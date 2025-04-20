use sbus_rs::channels_parsing;

use motor::Command;

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

        'outer: loop {
            // Read until header (`0x0f`) is detected.
            loop {
                self.uart.read(&mut buf[0..1]).await?;
                if buf[0] == 0x0f {
                    break;
                }
            }
            // Read the payload (23 bytes) and the footer (1 byte).
            self.uart.read(&mut buf[1..]).await?;
            if buf[24] != 0x00 {
                continue;
            }

            let channels = channels_parsing(&buf);
            for channel in channels.iter() {
                if *channel > 1800 || *channel < 240 {
                    continue 'outer;
                }
            }

            return Ok(Command {
                roll: scale_principal_axis(channels[0]),
                pitch: scale_principal_axis(channels[1]),
                yaw: scale_principal_axis(channels[3]),
                thrust: scale_thrust(channels[2]),
            });
        }
    }
}

fn scale_principal_axis(input: u16) -> f32 {
    // Set -1.0 and 1.0 explicitely to avoid rounding error.
    match input {
        SCALE_MID => 0.0,
        u16::MIN..SCALE_MID => {
            (SCALE_MID - input) as f32 / (SCALE_MID - SCALE_MIN) as f32 * -1.0f32
        }
        SCALE_MID..=u16::MAX => (input - SCALE_MID) as f32 / (SCALE_MAX - SCALE_MID) as f32,
    }
}

fn scale_thrust(input: u16) -> f32 {
    // Set 0.0 and 1.0 explicitely to avoid rounding error.
    match input {
        u16::MIN..SCALE_MIN => 0.0,
        SCALE_MIN..SCALE_MAX => (input - SCALE_MIN) as f32 / (SCALE_MAX - SCALE_MIN) as f32,
        SCALE_MAX..=u16::MAX => 1.0,
    }
}
