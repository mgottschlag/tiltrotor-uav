#![no_main]
#![no_std]

use core::fmt::Write;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;
use libm::fabsf;
use {defmt_rtt as _, panic_probe as _};

mod board;
mod display;
mod radio;
mod trace;

use board::{BatteryMonitor, Board, Direction, EnginePwm, EnginePwmType};
use display::Display;
use radio::Command;
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

/*macro_rules! print_no_defmt_error { // FIXME: print error (serde_cbor::Error does not implement defmt::Format) -> migrate to ciborium
    ( $( $msg:expr, $e:expr ),* ) => {
        $(
            let mut buf: String<256> = String::new();
            write!(&mut buf, "{:?}", $e).ok();
            error!("{}: {}", $msg, buf)
        )*
    };
}*/

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

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
            .send(display::Event::Command(Command::new()))
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
        if let Err(e) = spawner.spawn(poll_radio(
            radio,
            board.engines,
            &DISPLAY_EVENT_CHANNEL,
            &TRACE_EVENT_CHANNEL,
        )) {
            error!("Failed to spawn radio task: {}", e);
            DISPLAY_EVENT_CHANNEL
                .send(display::Event::Error(
                    display::ErrorCode::FailedToSpawnRadioTask,
                ))
                .await;
            trace_error!(e);
            loop {
                Timer::after_secs(1).await;
            }
        }
        info!("Done setting up radio");
    }

    loop {
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
pub async fn poll_radio(
    mut radio: Radio,
    mut engines: EnginePwmType,
    display_event_channel: &'static display::EventChannel,
    trace_event_channel: &'static trace::EventChannel,
) {
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
        display_event_channel
            .send(display::Event::Command(cmd.clone()))
            .await;

        // handle stop
        if cmd.pitch == 0.0 && cmd.roll == 0.0 {
            engines.update(Direction::Stop, Direction::Stop);
            continue;
        }

        let abs_pitch = fabsf(cmd.pitch);
        let abs_roll = fabsf(cmd.roll);

        // handle rotation
        // roll is anyway `!= 0.0`
        if cmd.pitch == 0.0 {
            match cmd.roll {
                _ if cmd.roll > 0.0 => {
                    engines.update(Direction::Backward(abs_roll), Direction::Forward(abs_roll))
                }
                _ if cmd.roll < 0.0 => {
                    engines.update(Direction::Forward(abs_roll), Direction::Backward(abs_roll))
                }
                _ => engines.update(Direction::Stop, Direction::Stop),
            };
            continue;
        }

        // handle movement
        match cmd.pitch {
            _ if cmd.pitch > 0.0 => engines.update(
                Direction::Forward((1.0 - abs_pitch) * abs_roll),
                Direction::Forward(abs_pitch),
            ),
            _ if cmd.pitch < 0.0 => engines.update(
                Direction::Backward(abs_roll),
                Direction::Backward((1.0 - abs_pitch) * abs_roll),
            ),
            _ => engines.update(Direction::Stop, Direction::Stop),
        }

        // write command to trace
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
