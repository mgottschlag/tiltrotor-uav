#![no_main]
#![no_std]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use motor::Command;
use panic_probe as _;
use stabilization::Kf;

mod board;
mod imu;
mod radio;

use board::Board;
use imu::Driver;
use imu::Imu;
use radio::Radio;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init(motor::car::Car::new());

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
        info!("thrust={}", thrust);

        Timer::after_millis(2).await;
    }
}

#[embassy_executor::task]
pub async fn poll_radio(mut radio: Radio) {
    info!("Polling from radio ...");
    let mut last_cmd = Command::new();
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
