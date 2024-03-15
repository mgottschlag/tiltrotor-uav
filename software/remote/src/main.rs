use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;

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

    let radio = Radio::new(opts.device).await;
    let reader = EventStream::new();

    enable_raw_mode().unwrap();
    let mut remote = Remote::new(radio, reader).await;
    remote.stop().await;
    remote.run().await;
    disable_raw_mode().unwrap();
}

struct Remote {
    radio: Radio,
    reader: crossterm::event::EventStream,
}

impl Remote {
    pub async fn new(radio: Radio, reader: crossterm::event::EventStream) -> Self {
        Remote { radio, reader }
    }

    async fn stop(&mut self) {
        self.radio.send(&mut [0 as i16; 4], &mut [0; 2]).await;
    }
    async fn run(&mut self) {
        let mut thrust: [i16; 4] = [0; 4];
        let mut pose: [i8; 2] = [0; 2];
        let mut last_event = SystemTime::now();
        let mut pressed = false;
        println!("Listening for packets:\r");
        loop {
            let delay = Delay::new(Duration::from_millis(10)).fuse();
            let event = self.reader.next().fuse();
            tokio::select! {
                packet = self.radio.receive() => {
                    let packet = packet.expect("could not receive packet");
                    println!("Received {:?} from {}\r", packet.payload, packet.pipe);
                },
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

                                // engine individual keys
                                // - 'z' + 'h' for engine 1
                                // - 'u' + 'j' for engine 1
                                // - 'i' + 'k' for engine 1
                                // - 'o' + 'l' for engine 1
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('z'),
                                }) => {
                                    thrust[0] += 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('h'),
                                }) => {
                                    thrust[0] -= 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('u'),
                                }) => {
                                    thrust[1] += 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('j'),
                                }) => {
                                    thrust[1] -= 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('i'),
                                }) => {
                                    thrust[2] += 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('k'),
                                }) => {
                                    thrust[2] -= 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('o'),
                                }) => {
                                    thrust[3] += 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('l'),
                                }) => {
                                    thrust[3] -= 10;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }

                                // control thrust
                                // - 'w' to go up
                                // - 's' to go down
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('w'),
                                }) => {
                                    thrust = thrust.map(|e| {e+10});
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('s'),
                                }) => {
                                    thrust = thrust.map(|e| {e-10});
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }

                                // control pose via arrow keys
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Up,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = 20;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Down,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = -20;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Left,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = 20;
                                    self.radio.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Right,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = -20;
                                    self.radio.send(&mut thrust, &mut pose).await;
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
                        self.radio.send(&mut thrust, &mut pose).await;
                    }
                },
            }
        }
    }
}
