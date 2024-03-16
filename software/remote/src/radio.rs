use nrf24l01_stick_driver::{
    Configuration, CrcMode, DataRate, Receiver, MAX_PAYLOAD_LEN, NRF24L01,
};
use protocol::{Command, Status};
use serde::ser::Serialize;
use serde_cbor::{de::from_mut_slice, ser::SliceWrite, Serializer};
use std::cmp::{max, min};
use std::path::PathBuf;

pub struct Radio {
    receiver: Receiver,
    cmd_queue: tokio::sync::mpsc::Receiver<Command>,
}

impl Radio {
    pub async fn new(device: PathBuf, cmd_queue: tokio::sync::mpsc::Receiver<Command>) -> Self {
        let mut config = Configuration::default();
        config.channel = 0x32;
        config.rate = DataRate::R2Mbps;
        config.power = 3;
        config.crc = Some(CrcMode::OneByte);
        config.auto_retransmit_delay_count = Some((250, 3));

        let mut nrf24l01 = NRF24L01::open_default(config, device.to_str().unwrap())
            .await
            .expect("could not open device");
        // data is received via ACK payloads -> no need to set any receive addresses
        nrf24l01
            .set_receive_addr(None, None, None, None, None)
            .await
            .expect("could not set receive address");

        let receiver = nrf24l01.receive().await.expect("could not start receiving");

        Radio {
            receiver,
            cmd_queue,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                packet = self.receiver.receive() => {
                    let packet = packet.expect("could not receive packet");
                    println!("Received {:?} from {}\r", packet.payload, packet.pipe);
                }
                Some(cmd) = self.cmd_queue.recv() => {
                    self.send(cmd).await
                },
            }
        }
    }

    pub async fn send(&mut self, cmd: Command) {
        /*thrust = thrust.map(|e| min(max(e, 0), 255));
        let cmd = Command {
            thrust: thrust.map(|e| min(max(e, 0), 255) as u8),
            pose: pose.map(|e| min(max(e, -90), 90) as i8),
        };*/

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
                println!("Received ACK payload: {data:?}. len={size}\r");
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
                println!("Did not receive ACK payload.\r");
            }
            Err(e) => println!("could not send: {e:?}\r"),
        }
    }
}
