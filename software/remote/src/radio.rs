use nrf24l01_stick_driver::{
    Configuration, CrcMode, DataRate, ReceivedPacket, Receiver, MAX_PAYLOAD_LEN, NRF24L01,
};
use protocol::{Command, Status};
use serde::ser::Serialize;
use serde_cbor::{de::from_mut_slice, ser::SliceWrite, Serializer};
use std::cmp::{max, min};
use std::path::PathBuf;

pub struct Radio {
    receiver: Receiver,
    last_cmd: Command,
}

impl Radio {
    pub async fn new(device: PathBuf) -> Self {
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

        let last_cmd = Command {
            thrust: [0; 4],
            pose: [0; 2],
        };

        Radio { receiver, last_cmd }
    }

    pub async fn receive(&mut self) -> Result<ReceivedPacket, nrf24l01_stick_driver::Error> {
        return self.receiver.receive().await;
    }

    pub async fn send(&mut self, thrust: &mut [i16; 4], pose: &mut [i8; 2]) {
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
}
