#![no_main]
#![no_std]

use core::fmt::Write;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;
use motor::Command;
use {defmt_rtt as _, panic_probe as _};

mod board;

use board::Board;

macro_rules! trace_error {
    ( $( $e:expr ),* ) => {
        $(
            let mut buf: String<256> = String::new();
            write!(&mut buf, "{:?}", $e).ok();
            TRACE_EVENT_CHANNEL.send(trace::Event::Error(buf)).await;
        )*
    };
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting ...");
    let _board = Board::init(motor::car::Car::new());

    loop {
        info!("Running ...");
        Timer::after_secs(1).await;
    }
}
