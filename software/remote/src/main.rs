use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use protocol::Command;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;
use tokio::sync::mpsc;

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

    #[structopt(short = "b", long)]
    offline: bool,
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();
    println!("{opts:?}");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
    let mut radio = Radio::new(opts.device, cmd_rx).await;
    tokio::spawn(async move { radio.run().await });

    let reader = EventStream::new();

    enable_raw_mode().unwrap();
    let mut remote = Remote::new(cmd_tx, reader).await;
    remote.stop().await;
    remote.run().await;
    disable_raw_mode().unwrap();
}

struct Remote {
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
    reader: crossterm::event::EventStream,
}

impl Remote {
    pub async fn new(
        cmd_tx: tokio::sync::mpsc::Sender<Command>,
        reader: crossterm::event::EventStream,
    ) -> Self {
        Remote { cmd_tx, reader }
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
        let mut thrust: [u8; 4] = [0; 4];
        let mut pose: [i8; 2] = [0; 2];
        let mut last_event = SystemTime::now();
        let mut pressed = false;
        println!("Listening for packets:\r");
        loop {
            let delay = Delay::new(Duration::from_millis(10)).fuse();
            let event = self.reader.next().fuse();
            tokio::select! {
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            //println!("Event::{:?}\r", event);
                            match event {
                                // add newline to terminal
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Enter,
                                }) => {
                                    println!("\r");
                                }

                                // stop remote
                                // - ESC
                                // - CTRL+C
                                // - CTRL+D
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Esc,
                                }) | Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::CONTROL,
                                    code: KeyCode::Char('c'),
                                }) | Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::CONTROL,
                                    code: KeyCode::Char('d'),
                                }) => {
                                    self.stop().await;
                                    break;
                                },

                                // control thrust
                                // - 'w' to go up
                                // - 's' to go down
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('w'),
                                }) => {
                                    thrust = thrust.map(|e| {e+10});
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('s'),
                                }) => {
                                    thrust = thrust.map(|e| {e-10});
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }

                                // control pose via arrow keys
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Up,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = 20;
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Down,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = -20;
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Left,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = 20;
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Right,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = -20;
                                    self.cmd_tx.send(Command {
                                        thrust: thrust,
                                        pose: pose,
                                    }).await.unwrap();
                                }

                                _ => {},
                            }
                        }
                        Some(Err(e)) => println!("Error: {e:?}\r"),
                        None => break,
                    }
                }
                _ = delay => {
                    if pressed && last_event.elapsed().unwrap() > Duration::new(0, 500_000_000) {
                        pressed = false;
                        pose = [0; 2];
                        self.cmd_tx.send(Command {
                            thrust: thrust,
                            pose: pose,
                        }).await.unwrap();
                    }
                },
            }
        }
    }
}
