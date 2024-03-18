#![no_main]
#![no_std]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

mod board;
mod radio;

use board::{Board, EnginePwm};
use radio::{Radio, RadioIrq};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

    info!("Setting up radio ...");
    let radio = Radio::init(board.radio_spi, board.radio_cs, board.radio_ce);
    info!("Done setting up radio");

    spawner
        .spawn(radio_interrupt(radio, board.radio_irq, board.engines))
        .unwrap();

    loop {
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
pub async fn radio_interrupt(mut radio: Radio, mut radio_irq: RadioIrq, engines: EnginePwm) {
    loop {
        radio_irq.wait_for_low().await;
        let mut status = protocol::Status { r: 0.5, p: 2.0 };
        match radio.poll(&status) {
            None => {}
            Some(cmd) => {
                info!("Got command: {}", cmd)
            }
        }
    }
}
