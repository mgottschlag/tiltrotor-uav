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

use board::{BatteryMonitor, Board, Direction, EnginePwm};
use display::{Event, EventChannel};
use radio::{Radio, RadioIrq};

static DISPLAY_EVENT_CHANNEL: EventChannel = Channel::new();
static STATUS: Mutex<CriticalSectionRawMutex, Status> = Mutex::new(Status {
    r: 0.0,
    p: 0.0,
    b: 0.0,
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ...");
    let board = Board::init();

    info!("Setting up display ...");
    spawner
        .spawn(display::handle(board.display_i2c, &DISPLAY_EVENT_CHANNEL))
        .unwrap();
    DISPLAY_EVENT_CHANNEL
        .send(Event::Command(Command::new()))
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
    event_channel: &'static EventChannel,
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
                event_channel.send(Event::Command(cmd.clone())).await;

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
            }
        }
    }
}

#[embassy_executor::task]
pub async fn battery_monitor(
    mut battery_monitor: BatteryMonitor,
    event_channel: &'static EventChannel,
) {
    loop {
        let voltage = battery_monitor.read();
        {
            let mut status_unlocked = STATUS.lock().await;
            status_unlocked.b = voltage;
        }
        event_channel.send(Event::Battery(voltage)).await;
        Timer::after_secs(10).await;
    }
}
