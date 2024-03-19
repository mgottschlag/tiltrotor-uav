use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use protocol::Command;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::sync::mpsc;

mod keyboard;
mod radio;

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
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();
    println!("{opts:?}");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
    let mut radio = Radio::new(opts.device, cmd_rx).await;
    tokio::spawn(async move { radio.run().await });

    let mut remote = Remote::new(cmd_tx).await;
    remote.run().await;
    disable_raw_mode().unwrap();
}

struct Remote {
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
}

impl Remote {
    pub async fn new(cmd_tx: tokio::sync::mpsc::Sender<Command>) -> Self {
        let mut remote = Remote { cmd_tx };
        remote.stop().await;
        enable_raw_mode().unwrap();

        remote
    }

    async fn stop(&mut self) {
        self.cmd_tx
            .send(Command {
                thrust: [0; 4],
                pose: [0; 2],
            })
            .await
            .unwrap();
    }

    async fn run(&mut self) {
        keyboard::run(&self.cmd_tx).await;
    }
}
