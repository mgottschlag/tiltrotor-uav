#![no_main]
#![no_std]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;
use stabilization::Kf;

mod board;
mod imu;
mod radio;

use board::Board;
use board::Command;
use board::UsbClass;
use board::UsbDevice;
use imu::Driver;
use imu::Imu;
use radio::Radio;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

    info!("Setting up usb ...");
    if let Err(e) = spawner.spawn(run_usb(board.usb_device)) {
        error!("Failed to spawn usb run task: {}", e);
        panic!()
    }
    if let Err(e) = spawner.spawn(poll_usb(board.usb_class)) {
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
        let (gyro, accel) = imu.get_rotations();
        let thrust = kf.update(gyro, accel);
        //info!("thrust={}", thrust);

        Timer::after_millis(100).await;
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
        last_cmd = cmd;
    }
}

#[embassy_executor::task]
async fn run_usb(mut usb_device: UsbDevice) {
    info!("Running usb device ...");
    usb_device.run().await;
}

#[embassy_executor::task]
async fn poll_usb(mut usb_class: UsbClass) {
    info!("Waiting for usb connection ...");
    usb_class.wait_connection().await;
    info!("Usb connected");
    let mut buf = [0; 64];
    loop {
        let n = usb_class.read_packet(&mut buf).await.unwrap();
        let data = &buf[..n];
        info!("data: {:x}", data);
        usb_class.write_packet(data).await.unwrap();
    }
}
