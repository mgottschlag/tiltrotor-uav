#![no_main]
#![no_std]

use core::fmt::Write;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

mod board;
mod display;
mod radio;
mod trace;

use board::{BatteryMonitor, Board, EnginePwm, EnginePwmType};
use display::Display;
use radio::Radio;

static TRACE_EVENT_CHANNEL: trace::EventChannel = Channel::new();
static DISPLAY_EVENT_CHANNEL: display::EventChannel = Channel::new();

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
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init(motor::Car::new());

    #[cfg(feature = "display")]
    {
        info!("Setting up display ...");
        let display = match Display::init(board.display_i2c) {
            Ok(display) => display,
            Err(e) => {
                error!("Failed to init display: {}", e);
                loop {
                    Timer::after_secs(1).await;
                }
            }
        };
        if let Err(e) = spawner.spawn(display::run(display, &DISPLAY_EVENT_CHANNEL)) {
            error!("Failed to spawn display event task: {}", e);
            loop {
                Timer::after_secs(1).await;
            }
        }
        DISPLAY_EVENT_CHANNEL
            .send(display::Event::Command(motor::Command::new()))
            .await;
    }

    #[cfg(feature = "sd-trace")]
    {
        info!("Setting up trace ...");
        if let Err(e) = spawner.spawn(trace::run(
            board.storage_spi,
            board.storage_cs,
            &TRACE_EVENT_CHANNEL,
        )) {
            error!("Failed to spawn trace event task: {}", e);
            DISPLAY_EVENT_CHANNEL
                .send(display::Event::Error(
                    display::ErrorCode::FailedToSpawnTraceTask,
                ))
                .await;
            loop {
                Timer::after_secs(1).await;
            }
        }
    }

    #[cfg(feature = "battery-monitor")]
    {
        info!("Setting up battery monitor ...");
        if let Err(e) = spawner.spawn(battery_monitor(
            board.battery_monitor,
            &DISPLAY_EVENT_CHANNEL,
        )) {
            error!("Failed to spawn battery monitor task: {}", e);
            DISPLAY_EVENT_CHANNEL
                .send(display::Event::Error(
                    display::ErrorCode::FailedToSpawnBatteryMonitorTask,
                ))
                .await;
            trace_error!(e);
            loop {
                Timer::after_secs(1).await;
            }
        }
    }

    #[cfg(feature = "radio")]
    {
        info!("Setting up radio ...");
        let radio = Radio::init(board.radio_uart);
        info!("Done setting up radio");
        poll_radio(
            radio,
            board.engines,
            &DISPLAY_EVENT_CHANNEL,
            &TRACE_EVENT_CHANNEL,
        )
        .await;
    }

    loop {
        Timer::after_secs(1).await;
    }
}

pub async fn poll_radio<M: motor::Type>(
    mut radio: Radio,
    mut engines: EnginePwmType<M>,
    display_event_channel: &'static display::EventChannel,
    trace_event_channel: &'static trace::EventChannel,
) {
    info!("Polling radio ...");
    loop {
        let cmd = match radio.next().await {
            Ok(data) => data,
            Err(e) => {
                error!("Failed get get data from radio: {}", e);
                continue;
            }
        };

        info!("Got command: {}", cmd);

        // display command
        #[cfg(feature = "display")]
        display_event_channel
            .send(display::Event::Command(cmd.clone()))
            .await;

        engines.update(&cmd);

        // write command to trace
        #[cfg(feature = "sd-trace")]
        trace_event_channel
            .send(trace::Event::Command(cmd.clone()))
            .await;
    }
}

#[embassy_executor::task]
pub async fn battery_monitor(
    mut battery_monitor: BatteryMonitor,
    event_channel: &'static display::EventChannel,
) {
    loop {
        let voltage = battery_monitor.read();
        event_channel.send(display::Event::Battery(voltage)).await;
        Timer::after_secs(10).await;
    }
}
