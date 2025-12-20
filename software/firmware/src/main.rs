#![no_main]
#![no_std]

use defmt::{error, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::channel::Sender;
use embassy_time::Timer;
use panic_probe as _;
use stabilization::Kf;

mod board;
mod imu;
mod radio;

use board::Board;
use board::Command;
use board::UsbDevice;
use board::UsbReceiver;
use imu::Driver;
use imu::Imu;
use radio::Radio;

static CMD_CHANNEL: Channel<ThreadModeRawMutex, Command, 16> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

    info!("Setting up usb ...");
    let (_usb_sender, usb_receiver) = board.usb_class.split();
    if let Err(e) = spawner.spawn(run_usb(board.usb_device)) {
        error!("Failed to spawn usb run task: {}", e);
        panic!()
    }
    if let Err(e) = spawner.spawn(poll_usb(usb_receiver)) {
        error!("Failed to spawn usb poll task: {}", e);
        panic!()
    }
    info!("Done setting up usb");

    info!("Setting up radio ...");
    let radio = Radio::init(board.radio_uart);
    if let Err(e) = spawner.spawn(poll_radio(radio)) {
        error!("Failed to spawn radio task: {}", e);
        panic!()
    }
    info!("Done setting up radio");

    info!("Setting up IMU ...");
    let imu_driver = imu::Icm20689::init(board.imu_spi, board.imu_cs);
    let mut imu = Imu::init(imu_driver);
    let mut kf = Kf::new();
    info!("Done setting up IMU");

    loop {
        let cmd = CMD_CHANNEL.receive().await;
        info!("Command: {}", cmd);
        /*let (gyro, accel) = imu.get_rotations();
        let _thrust = kf.update(gyro, accel);
        //info!("thrust={}", thrust);

        Timer::after_millis(2).await;*/
    }
}

#[embassy_executor::task]
async fn poll_radio(mut radio: Radio) {
    info!("Polling from radio ...");
    let mut last_cmd = Command::Remote {
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
        thrust: 0.0,
    };
    loop {
        let cmd = match radio.next().await {
            Ok(data) => data,
            Err(embassy_stm32::usart::Error::Noise) => continue,
            Err(e) => {
                error!("Failed get get data from radio: {}", e);
                continue;
            }
        };
        if cmd == last_cmd {
            continue;
        }
        info!("Got command: {}", cmd);
        CMD_CHANNEL.send(cmd.clone()).await;
        last_cmd = cmd;
    }
}

#[embassy_executor::task]
async fn run_usb(mut usb_device: UsbDevice) {
    info!("Running usb device ...");
    usb_device.run().await;
}

#[embassy_executor::task]
async fn poll_usb(mut usb_class: UsbReceiver) {
    info!("Waiting for usb connection ...");
    usb_class.wait_connection().await;
    info!("Usb connected");
    let mut buf = [0; 64];
    loop {
        let n = usb_class.read_packet(&mut buf).await.unwrap();
        if n < 4 {
            warn!("Only got {} bytes via usb: {}", n, &buf[..n]);
            continue;
        }
        let cmd = Command::MotorDebug {
            m1: (f32::from(buf[0]) / 255.0).max(0.0).min(1.0),
            m2: (f32::from(buf[1]) / 255.0).max(0.0).min(1.0),
            m3: (f32::from(buf[2]) / 255.0).max(0.0).min(1.0),
            m4: (f32::from(buf[3]) / 255.0).max(0.0).min(1.0),
        };
        CMD_CHANNEL.send(cmd).await;
    }
}
