#![no_main]
#![no_std]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_time::Timer;
use motor::Command;
use {defmt_rtt as _, panic_probe as _};

mod board;
mod radio;

use board::Board;
use radio::Radio;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init(motor::car::Car::new());

    info!("Setting up radio ...");
    let radio = Radio::init(board.radio_uart);
    info!("Done setting up radio");
    poll_radio(radio).await;

    loop {
        Timer::after_secs(1).await;
    }
}

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
