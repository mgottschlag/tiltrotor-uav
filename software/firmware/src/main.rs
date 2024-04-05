#![no_main]
#![no_std]

use defmt::*;
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use libm::fabsf;
use protocol::Command;
use {defmt_rtt as _, panic_probe as _};

mod board;
mod display;
mod radio;

use board::{Board, Direction, EnginePwm};
use display::{Event, EventChannel};
use radio::{Radio, RadioIrq};

static DISPLAY_EVENT_CHANNEL: EventChannel = Channel::new();

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
        let status = protocol::Status { r: 0.5, p: 2.0 };
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
