use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use protocol::Command;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::sync::mpsc;

mod gamepad;
mod keyboard;
mod radio;

use gamepad::Gamepad;
use radio::Radio;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opts {
    #[structopt(
        short = "d",
        long,
        parse(from_os_str),
        default_value = "/dev/ttyUSB_nrf24l01"
    )]
    device: PathBuf,

    #[structopt(short = "o", long)]
    offline: bool,
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();
    println!("Opts: {opts:?}");

    // init command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(32);
    cmd_tx
        .send(Command {
            thrust: [0; 4],
            pose: [0; 2],
        })
        .await
        .unwrap();

    match Gamepad::init() {
        None => {
            println!("Did not find any gamepads - falling back to keyboard");
            tokio::spawn(async move {
                enable_raw_mode().unwrap();
                keyboard::run(&cmd_tx).await;
                disable_raw_mode().unwrap();
            });
        }
        Some(mut gamepad) => {
            println!("Found at least one gamepad - waiting for input");
            tokio::task::spawn_blocking(move || gamepad.run(&cmd_tx));
        }
    };

    match opts.offline {
        true => {
            println!("Waiting for commands in offline mode ...");
            loop {
                let cmd = cmd_rx.recv().await.unwrap();
                println!("Got {cmd:?}");
            }
        }
        false => {
            // init radio
            let mut radio = Radio::new(opts.device, cmd_rx).await;
            println!("Waiting for commands ...");
            radio.run().await;
        }
    }
}
