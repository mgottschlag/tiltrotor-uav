use sbus_rs::channels_parsing;

use crate::board::RadioUart;

const SCALE_MIN: u16 = 240;
const SCALE_MID: u16 = 1024;
const SCALE_MAX: u16 = 1800;

pub struct Radio {
    uart: RadioUart,
}

impl Radio {
    pub fn init(uart: RadioUart) -> Self {
        Self { uart }
    }

    pub async fn next(&mut self) -> Result<Command, embassy_stm32::usart::Error> {
        let mut buf = [0u8; 25];

        // Read until header (`0x0f`) is detected.
        loop {
            self.uart.read(&mut buf[0..1]).await?;
            if buf[0] == 0x0f {
                break;
            }
        }
        // Read the payload (23 bytes) and the footer 1 bytes.
        self.uart.read(&mut buf[1..]).await?;
        let channels = channels_parsing(&buf);

        Ok(Command {
            roll: scale(channels[3]),
            pitch: scale(channels[2]),
            yaw: scale(channels[0]),
            thrust: scale(channels[1]),
        })
    }
}

fn scale(input: u16) -> f32 {
    match input {
        SCALE_MID => 0.0,
        u16::MIN..SCALE_MID => (input - SCALE_MID) as f32 / (SCALE_MID - SCALE_MIN) as f32,
        SCALE_MID..=u16::MAX => (input - SCALE_MID) as f32 / (SCALE_MAX - SCALE_MID) as f32,
    }
}

#[derive(Clone, Debug, defmt::Format)]
pub struct Command {
    pub roll: f32,   // [-1.0 .. 1.0]
    pub pitch: f32,  // [-1.0 .. 1.0]
    pub yaw: f32,    // [-1.0 .. 1.0]
    pub thrust: f32, // [0.0 .. 1.0]
}

impl Command {
    pub fn new() -> Self {
        Command {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            thrust: 0.0,
        }
    }
}
