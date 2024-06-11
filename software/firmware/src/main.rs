#![no_main]
#![no_std]

use core::fmt::Write;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use heapless::String;
use libm::fabsf;
use protocol::{Command, Status};
use {defmt_rtt as _, panic_probe as _};

mod board;
mod display;
mod radio;
mod trace;

use board::{BatteryMonitor, Board, Direction, EnginePwm, EnginePwmType};
use display::Display;
use radio::{Radio, RadioIrq};

static TRACE_EVENT_CHANNEL: trace::EventChannel = Channel::new();
static DISPLAY_EVENT_CHANNEL: display::EventChannel = Channel::new();
static STATUS: Mutex<CriticalSectionRawMutex, Status> = Mutex::new(Status {
    roll: 0.0,
    pitch: 0.0,
    battery: 0.0,
});

macro_rules! trace_error {
    ( $( $e:expr ),* ) => {
        $(
            let mut buf: String<256> = String::new();
            write!(&mut buf, "{:?}", $e).ok();
            TRACE_EVENT_CHANNEL.send(trace::Event::Error(buf)).await;
        )*
    };
}

macro_rules! print_no_defmt_error { // FIXME: print error (serde_cbor::Error does not implement defmt::Format) -> migrate to ciborium
    ( $( $msg:expr, $e:expr ),* ) => {
        $(
            let mut buf: String<256> = String::new();
            write!(&mut buf, "{:?}", $e).ok();
            error!("{}: {}", $msg, buf)
        )*
    };
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

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

    {
        info!("Setting up radio ...");
        let radio = match Radio::init(board.radio_spi, board.radio_cs, board.radio_ce) {
            Ok(radio) => radio,
            Err(e) => {
                print_no_defmt_error!("Failed to init radio", e);
                DISPLAY_EVENT_CHANNEL
                    .send(display::Event::Error(display::ErrorCode::FailedToInitRadio))
                    .await;
                trace_error!(e);
                loop {
                    Timer::after_secs(1).await;
                }
            }
        };
        if let Err(e) = spawner.spawn(radio_interrupt(
            radio,
            board.radio_irq,
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
pub async fn radio_interrupt(
    mut radio: Radio,
    mut radio_irq: RadioIrq,
    mut engines: EnginePwmType,
    display_event_channel: &'static display::EventChannel,
    trace_event_channel: &'static trace::EventChannel,
) {
    loop {
        radio_irq.wait_for_low().await;

        // Clone latest status to avoid hanging too long in blocking mode while polling from radio.
        let status = STATUS.lock().await.clone();

        let cmd_opt = match radio.poll(&status) {
            Ok(cmd) => cmd,
            Err(e) => {
                print_no_defmt_error!("Failed to handle receive of incoming message", e);
                continue;
            }
        };
        match cmd_opt {
            None => {}
            Some(cmd) => {
                info!("Got command: {}", cmd);

                // display command
                display_event_channel
                    .send(display::Event::Command(cmd.clone()))
                    .await;

                // handle stop
                if cmd.pose.into_iter().all(|e| e == 0.0) {
                    engines.update(Direction::Stop, Direction::Stop);
                    continue;
                }

                let duty_y = fabsf(cmd.pose[1]);
                let duty_x = fabsf(cmd.pose[0]);

                // handle rotation
                if cmd.pose[0] == 0.0 {
                    match cmd.pose[1] {
                        _ if cmd.pose[1] > 0.0 => {
                            engines.update(Direction::Backward(duty_y), Direction::Forward(duty_y))
                        }
                        _ if cmd.pose[1] < 0.0 => {
                            engines.update(Direction::Forward(duty_y), Direction::Backward(duty_y))
                        }
                        _ => engines.update(Direction::Stop, Direction::Stop),
                    };
                    continue;
                }

                // handle movement
                match cmd.pose[0] {
                    _ if cmd.pose[0] > 0.0 => engines.update(
                        Direction::Forward((1.0 - duty_y) * duty_x),
                        Direction::Forward(duty_x),
                    ),
                    _ if cmd.pose[0] < 0.0 => engines.update(
                        Direction::Backward(duty_x),
                        Direction::Backward((1.0 - duty_y) * duty_x),
                    ),
                    _ => engines.update(Direction::Stop, Direction::Stop),
                }

                // write command to trace
                trace_event_channel
                    .send(trace::Event::Command(cmd.clone()))
                    .await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn battery_monitor(
    mut battery_monitor: BatteryMonitor,
    event_channel: &'static display::EventChannel,
) {
    loop {
        let voltage = battery_monitor.read();
        {
            let mut status_unlocked = STATUS.lock().await;
            status_unlocked.battery = voltage;
        }
        event_channel.send(display::Event::Battery(voltage)).await;
        Timer::after_secs(10).await;
    }
}
