use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use nrf24l01_stick_driver::Receiver;
use protocol::{Command, Status};
use serde::ser::Serialize;
use serde_cbor::{de::from_mut_slice, ser::SliceWrite, Serializer};
use std::cmp::{max, min};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;

use nrf24l01_stick_driver::{Configuration, CrcMode, DataRate, MAX_PAYLOAD_LEN, NRF24L01};

struct Remote {
    receiver: Receiver,
    reader: crossterm::event::EventStream,
    last_cmd: Command,
}

impl Remote {
    pub async fn new(receiver: Receiver, reader: crossterm::event::EventStream) -> Self {
        let last_cmd = Command {
            thrust: [0; 4],
            pose: [0; 2],
        };
        Remote {
            receiver,
            reader,
            last_cmd,
        }
    }

    async fn send(&mut self, thrust: &mut [i16; 4], pose: &mut [i8; 2]) {
        *thrust = thrust.map(|e| min(max(e, 0), 255));
        let cmd = Command {
            thrust: thrust.map(|e| min(max(e, 0), 255) as u8),
            pose: pose.map(|e| min(max(e, -90), 90) as i8),
        };

        if cmd.eq(&self.last_cmd) {
            return;
        }
        self.last_cmd = cmd;
        println!("{:?}\r", self.last_cmd);

        let mut buf = [0u8; 32];
        let writer = SliceWrite::new(&mut buf[..]);
        let mut ser = Serializer::new(writer);
        self.last_cmd.serialize(&mut ser).ok();
        let writer = ser.into_inner();
        let size = writer.bytes_written();
        if size > MAX_PAYLOAD_LEN {
            println!("ERROR: maximum payload size exeeded ({})\r", size);
            return;
        }
        match self
            .receiver
            .send(
                (&[0x44u8, 0x72u8, 0x6fu8, 0x6eu8, 0x65u8][..]).into(),
                &buf[0..size],
            )
            .await
        {
            Ok(Some(ack_payload)) => {
                let mut data = ack_payload.payload;
                let size = data.len();
                println!("Received ACK payload: {:?}. len={}\r", data, size);
                match from_mut_slice::<Status>(&mut data[..size]) {
                    Err(err) => {
                        println!("err: {}\r", err)
                    }
                    Ok(status) => {
                        println!("status: r={}, p={}\r", status.r, status.p)
                    }
                }
            }
            Ok(None) => {
                println!("Received no ACK payload.\r");
            }
            Err(e) => println!("could not send: {:?}\r", e),
        }
    }

    async fn stop(&mut self) {
        self.send(&mut [0 as i16; 4], &mut [0; 2]).await;
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
                packet = self.receiver.receive() => {
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
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('h'),
                                }) => {
                                    thrust[0] -= 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('u'),
                                }) => {
                                    thrust[1] += 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('j'),
                                }) => {
                                    thrust[1] -= 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('i'),
                                }) => {
                                    thrust[2] += 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('k'),
                                }) => {
                                    thrust[2] -= 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('o'),
                                }) => {
                                    thrust[3] += 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('l'),
                                }) => {
                                    thrust[3] -= 10;
                                    self.send(&mut thrust, &mut pose).await;
                                }

                                // control thrust
                                // - 'w' to go up
                                // - 's' to go down
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('w'),
                                }) => {
                                    thrust = thrust.map(|e| {e+10});
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Char('s'),
                                }) => {
                                    thrust = thrust.map(|e| {e-10});
                                    self.send(&mut thrust, &mut pose).await;
                                }

                                // control pose via arrow keys
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Up,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = 20;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Down,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[0] = -20;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Left,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = 20;
                                    self.send(&mut thrust, &mut pose).await;
                                }
                                Event::Key(KeyEvent{
                                    modifiers: KeyModifiers::NONE,
                                    code: KeyCode::Right,
                                }) => {
                                    last_event = SystemTime::now();
                                    pressed = true;
                                    pose[1] = -20;
                                    self.send(&mut thrust, &mut pose).await;
                                }

                                _ => {},
                            }
                        }
                        Some(Err(e)) => println!("Error: {:?}\r", e),
                        None => break,
                    }
                }
                _ = delay => {
                    if pressed && last_event.elapsed().unwrap() > Duration::new(0, 500_000_000) {
                        pressed = false;
                        pose = [0; 2];
                        self.send(&mut thrust, &mut pose).await;
                    }
                },
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();
    println!("{:#?}", opts);

    let mut config = Configuration::default();
    config.channel = 0x32;
    config.rate = DataRate::R2Mbps;
    config.power = 3;
    config.crc = Some(CrcMode::OneByte);
    config.auto_retransmit_delay_count = Some((250, 3));

    let mut nrf24l01 =
        NRF24L01::open_default(config, &opts.device.into_os_string().into_string().unwrap())
            .await
            .expect("could not open device");
    // data is received via ACK payloads -> no need to set any receive addresses
    nrf24l01
        .set_receive_addr(None, None, None, None, None)
        .await
        .expect("could not set receive address");
    let receiver = nrf24l01.receive().await.expect("could not start receiving");

    let reader = EventStream::new();

    enable_raw_mode().unwrap();
    let mut remote = Remote::new(receiver, reader).await;
    remote.stop().await;
    remote.run().await;
    disable_raw_mode().unwrap();
}

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
