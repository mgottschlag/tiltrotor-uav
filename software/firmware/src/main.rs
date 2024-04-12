#![no_main]
#![no_std]

use defmt::*;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use libm::fabsf;
use protocol::{Command, Status};
use {defmt_rtt as _, panic_probe as _};

mod board;
mod display;
mod radio;
mod trace;

use board::{BatteryMonitor, Board, Direction, EnginePwm};
use radio::{Radio, RadioIrq};

static TRACE_EVENT_CHANNEL: trace::EventChannel = Channel::new();
static DISPLAY_EVENT_CHANNEL: display::EventChannel = Channel::new();
static STATUS: Mutex<CriticalSectionRawMutex, Status> = Mutex::new(Status {
    r: 0.0,
    p: 0.0,
    b: 0.0,
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

    info!("Setting up trace ...");
    spawner
        .spawn(trace::run(
            board.storage_spi,
            board.storage_cs,
            &TRACE_EVENT_CHANNEL,
        ))
        .unwrap();

    info!("Setting up display ...");
    spawner
        .spawn(display::run(board.display_i2c, &DISPLAY_EVENT_CHANNEL))
        .unwrap();
    DISPLAY_EVENT_CHANNEL
        .send(display::Event::Command(Command::new()))
        .await;

    info!("Setting up battery monitor ...");
    spawner
        .spawn(battery_monitor(
            board.battery_monitor,
            &DISPLAY_EVENT_CHANNEL,
        ))
        .unwrap();

    info!("Setting up radio ...");
    let radio = Radio::init(board.radio_spi, board.radio_cs, board.radio_ce);
    info!("Done setting up radio");

    spawner
        .spawn(radio_interrupt(
            radio,
            board.radio_irq,
            board.engines,
            &DISPLAY_EVENT_CHANNEL,
            &TRACE_EVENT_CHANNEL,
        ))
        .unwrap();

    loop {
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
pub async fn radio_interrupt(
    mut radio: Radio,
    mut radio_irq: RadioIrq,
    mut engines: EnginePwm,
    display_event_channel: &'static display::EventChannel,
    trace_event_channel: &'static trace::EventChannel,
) {
    loop {
        radio_irq.wait_for_low().await;

        // Clone latest status to avoid hanging too long in blocking mode while polling from radio.
        let status = STATUS.lock().await.clone();

        match radio.poll(&status) {
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
            status_unlocked.b = voltage;
        }
        event_channel.send(display::Event::Battery(voltage)).await;
        Timer::after_secs(10).await;
    }
}
