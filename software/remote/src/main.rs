use protocol::{Command, Status};
use serde::ser::Serialize;
use serde_cbor::de::from_mut_slice;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use nrf24l01_stick_driver::{Configuration, CrcMode, DataRate, NRF24L01};

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
    nrf24l01
        .set_receive_addr(
            Some((&[0xb3u8, 0xb3u8, 0xb3u8, 0xb3u8, 0x00u8] as &[u8]).into()),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("could not set receive address");
    let mut receive = nrf24l01.receive().await.expect("could not start receiving");

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    println!("Listening for packets:");
    loop {
        tokio::select! {
            packet = receive.receive() => {
                let packet = packet.expect("could not receive packet");
                println!("Received {:?} from {}", packet.payload, packet.pipe);
            },
            line = lines.next_line() => {
                let line = line.unwrap();
                if line.is_none() {
                    break;
                }
                let line: Vec<String> = line.unwrap().split(" ").map(str::to_string).collect();
                let data = match line.len() {
                    4 => Command{thrust: [line[0].parse::<u8>().unwrap(), line[1].parse::<u8>().unwrap(), line[2].parse::<u8>().unwrap(), line[3].parse::<u8>().unwrap()]},
                    _ => {let v = line[0].parse::<u8>().unwrap(); Command{thrust: [v, v, v, v]}}
                };

                let mut buf = [0u8; 32];
                let writer = SliceWrite::new(&mut buf[..]);
                let mut ser = Serializer::new(writer);
                data.serialize(&mut ser).ok();
                let writer = ser.into_inner();
                let size = writer.bytes_written();

                /*let payload = if line.as_bytes().len() > MAX_PAYLOAD_LEN {
                    &line.as_bytes()[0..MAX_PAYLOAD_LEN]
                } else {
                    line.as_bytes()
                };*/
                // TODO MAX_PAYLOAD_LEN warning

                match receive.send((&[0xe7u8, 0xe7u8, 0xe7u8, 0xe7u8, 0xe7u8][..]).into(), &buf[0..size]).await {
                    Ok(Some(ack_payload)) => {
                        let mut data = ack_payload.payload;
                        let size = data.len();
                        println!("Received ACK payload: {:?}. len={}", data, size);

                        match from_mut_slice::<Status>(&mut data[..size]) {
                            Err(err) => {println!("err: {}", err)}
                            Ok(status) => {println!("status: r={}, p={}", status.r, status.p)}
                        }
                    },
                    Ok(None) => {
                        println!("Received no ACK payload.");
                    },
                    Err(e) => println!("could not send: {:?}", e),
                }
            },

        }
    }
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
