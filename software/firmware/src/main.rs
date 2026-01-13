#![no_main]
#![no_std]

use defmt::{error, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_io_async::Write;
use panic_probe as _;
use stabilization::Kf;

mod board;
mod imu;
mod radio;

use board::Board;
use board::UsbDevice;
use board::UsbReceiver;
use imu::Driver;
use imu::Imu;
use protocol::Message;
use radio::Radio;

use crate::board::EscDriver;
use crate::board::UsbSender;

// Maybe needs to be improved later to some  double buffering with atomic pointer switching.
static THRUST: Mutex<CriticalSectionRawMutex, [f32; 4]> = Mutex::new([0.0; 4]);
static USB_CONNECTED: Mutex<CriticalSectionRawMutex, bool> = Mutex::new(false);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let mut board = Board::init();
    board.esc_driver.update([0.0; 4]);

    info!("Setting up usb ...");
    let (mut usb_sender, usb_receiver) = board.usb_class.split();
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
    let mut kf = Kf::new(0.002);
    //let mut kf = Kf::new(1.0);
    info!("Done setting up IMU");

    loop {
        let thrust_input;
        {
            let thrust_cmd = THRUST.lock().await;
            thrust_input = *thrust_cmd;
        }

        let (gyro, accel) = imu.get_rotations();
        let (rates, thrust) = kf.update(gyro, accel, thrust_input);
        info!(
            "thrust_input={}, thrust={}, rates={}",
            thrust_input, thrust, rates
        );

        {
            let usb_connected = USB_CONNECTED.lock().await;
            if *usb_connected {
                send_usb(
                    &mut usb_sender,
                    &Message::ImuData {
                        gyro,
                        accel,
                        rates,
                        thrust_input,
                        thrust,
                    },
                )
                .await;
            }
        }

        Timer::after_millis(2).await;
        //Timer::after_millis(1000).await;
    }
}

#[embassy_executor::task]
async fn poll_radio(mut radio: Radio) {
    info!("Polling from radio ...");
    let mut last_cmd = Message::Command {
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
        match cmd {
            Message::Command {
                roll: _,
                pitch: _,
                yaw: _,
                thrust,
            } => {
                let mut thrust_cmd = THRUST.lock().await;
                *thrust_cmd = [thrust; 4];
            }
            _ => {}
        }
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
    {
        let mut usb_connected = USB_CONNECTED.lock().await;
        *usb_connected = true;
    }
    info!("Usb connected");
    let mut buf = [0; 64];
    loop {
        let n = usb_class.read_packet(&mut buf).await.unwrap();
        match protocol::decode(&buf[..n]) {
            Ok(cmd) => {
                info!("Got command: {}", cmd);
                match cmd {
                    Message::MotorDebug { thrust } => {
                        let mut thrust_cmd = THRUST.lock().await;
                        *thrust_cmd = thrust;
                    }
                    _ => {}
                }
            }
            Err(_) => warn!("Failed to decode message"), // TODO: print actual error
        };
    }
}

async fn send_usb(sender: &mut UsbSender, msg: &Message) {
    let mut buf: [u8; 128] = [0; 128];
    let data = match protocol::encode(msg, &mut buf[1..]) {
        Ok(data) => data,
        Err(_e) => {
            error!("Failed to encode message"); // TODO: print actual error
            return;
        }
    };
    // TODO: integrate into protocol
    if let Err(e) = sender.write(&[data.len() as u8]).await {
        warn!("Failed to send message header via usb: {}", e);
    }
    info!("Sending data: {:?}", data);
    if let Err(e) = sender.write(&data).await {
        warn!("Failed to send message via usb: {} (len={})", e, data.len());
    }
}
