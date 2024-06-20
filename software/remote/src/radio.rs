use nrf24l01_stick_driver::{
    Configuration, CrcMode, DataRate, Receiver, MAX_PAYLOAD_LEN, NRF24L01,
};
use protocol::{Command, Status};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("interface error")]
    Interface(#[from] nrf24l01_stick_driver::Error),
}

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
                    println!("Got {cmd:?}\r");

                    let max_retries = 10;
                    let mut retries = 0;
                    while retries < max_retries {
                        retries += 1;
                        match self.send(cmd.clone()).await {
                            Ok(_) => break,
                            Err(e) => println!("could not send: {e:?}\r"),
                        }
                        sleep(Duration::from_millis(100)).await;
                        println!("Doing resend ({}th fail)", retries+1);
                    }

                    if retries == max_retries {
                        println!("Maximum retries reached - clearing command queue and try to stop");
                        while !self.cmd_queue.is_empty() {
                            _ = self.cmd_queue.recv().await
                        }
                        println!("Queue cleared")
                    }

                },
            }
        }
    }

    pub async fn send(&mut self, mut cmd: Command) -> Result<(), Error> {
        cmd.thrust = cmd.thrust.map(|e| e.clamp(0, 255));
        cmd.pose = cmd.pose.map(|e| e.clamp(-1.0, 1.0));

        let size = minicbor::len(&cmd); // TODO: handle size >= MAX_PAYLOAD_LEN bytes
        let mut buf = [0u8; MAX_PAYLOAD_LEN];
        minicbor::encode(&cmd, buf.as_mut()).unwrap();

        match self
            .receiver
            .send(
                (&[0x44u8, 0x72u8, 0x6fu8, 0x6eu8, 0x65u8][..]).into(),
                &buf[..size],
            )
            .await
        {
            Ok(Some(ack_payload)) => {
                let data = ack_payload.payload;
                let size = data.len();
                println!("Received ACK payload: {data:?}. len={size}\r");

                let status: Result<Status, minicbor::decode::Error> = minicbor::decode(&data[..]);
                match status {
                    Ok(status) => println!(
                        "roll={}, pitch={}, battery={}\r",
                        status.roll, status.pitch, status.battery
                    ),
                    Err(e) => println!("ERR: failed to decode status: {e}"),
                }
            }
            Ok(None) => {
                println!("Did not receive ACK payload.\r");
            }
            Err(e) => return Err(Error::Interface(e)),
        }

        Ok(())
    }
}
