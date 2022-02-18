use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, StreamExt};
use protocol::{Command, Status};
use serde::ser::Serialize;
use serde_cbor::{de::from_mut_slice, ser::SliceWrite, Serializer};
use std::cmp::{max, min};
use std::path::PathBuf;
use structopt::StructOpt;

use nrf24l01_stick_driver::{Configuration, CrcMode, DataRate, MAX_PAYLOAD_LEN, NRF24L01};

async fn send(receive: &mut nrf24l01_stick_driver::Receiver, thrust: &mut [i16; 4]) {
    *thrust = thrust.map(|e| min(max(e, 0), 255));
    let cmd = Command {
        thrust: thrust.map(|e| min(max(e, 0), 255) as u8),
    };
    println!("{:?}\r", cmd);

    let mut buf = [0u8; 32];
    let writer = SliceWrite::new(&mut buf[..]);
    let mut ser = Serializer::new(writer);
    cmd.serialize(&mut ser).ok();
    let writer = ser.into_inner();
    let size = writer.bytes_written();

    if size > MAX_PAYLOAD_LEN {
        println!("ERROR: maximum payload size exeeded ({})\r", size);
        return;
    }

    match receive
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

async fn stop(receive: &mut nrf24l01_stick_driver::Receiver) {
    send(receive, &mut [0 as i16; 4]).await;
}

async fn run(
    receive: &mut nrf24l01_stick_driver::Receiver,
    reader: &mut crossterm::event::EventStream,
) {
    let mut thrust: [i16; 4] = [0; 4];
    println!("Listening for packets:\r");
    loop {
        let event = reader.next().fuse();
        tokio::select! {
            packet = receive.receive() => {
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
                            }) => break,

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
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('h'),
                            }) => {
                                thrust[0] -= 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('u'),
                            }) => {
                                thrust[1] += 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('j'),
                            }) => {
                                thrust[1] -= 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('i'),
                            }) => {
                                thrust[2] += 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('k'),
                            }) => {
                                thrust[2] -= 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('o'),
                            }) => {
                                thrust[3] += 10;
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('l'),
                            }) => {
                                thrust[3] -= 10;
                                send(receive, &mut thrust).await;
                            }

                            // control thrust
                            // - 'w' to go up
                            // - 's' to go down
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('w'),
                            }) => {
                                thrust = thrust.map(|e| {e+10});
                                send(receive, &mut thrust).await;
                            }
                            Event::Key(KeyEvent{
                                modifiers: KeyModifiers::NONE,
                                code: KeyCode::Char('s'),
                            }) => {
                                thrust = thrust.map(|e| {e-10});
                                send(receive, &mut thrust).await;
                            }

                            _ => {},
                        }

                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break,
                }
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
    let mut receive = nrf24l01.receive().await.expect("could not start receiving");

    let mut reader = EventStream::new();

    stop(&mut receive).await;
    enable_raw_mode().unwrap();
    run(&mut receive, &mut reader).await;
    disable_raw_mode().unwrap();
    stop(&mut receive).await;
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
