//! Implements the USB protocol.
//!
//! +-------------------------------+
//! |            FRAME              |
//! +---------------+---------------+
//! |    HEADER     |    PAYLOAD    |
//! +---------------+---------------+
//!
#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, defmt::Format)]
pub enum Message {
    Command {
        roll: f32,   // [-1.0 .. 1.0]
        pitch: f32,  // [-1.0 .. 1.0]
        yaw: f32,    // [-1.0 .. 1.0]
        thrust: f32, // [ 0.0 .. 1.0]
    },
    MotorDebug {
        thrust: [f32; 4], // [0.0 .. 1.0]
    },
    ImuData {
        gyro: [f32; 3],
        accel: [f32; 3],
        rates: [f32; 2],
        thrust_input: [f32; 4], // [0.0 .. 1.0]
        thrust: [f32; 4],       // [0.0 .. 1.0]
    },
}

const HEADER_LEN: usize = 1;

pub fn encode(msg: &Message, buf: &mut [u8]) -> Result<usize, postcard::Error> {
    let (header_buf, payload_buf) = buf.split_at_mut(1);
    let payload = postcard::to_slice(msg, payload_buf)?;
    let frame_len = payload.len() + HEADER_LEN;
    assert!(frame_len <= u8::MAX as usize);
    header_buf[0] = frame_len as u8;
    Ok(frame_len)
}

pub fn decode(data: &[u8]) -> Result<Message, postcard::Error> {
    let frame_len: usize = data[0] as usize;
    postcard::from_bytes(&data[HEADER_LEN..frame_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_command() {
        let msg = Message::Command {
            roll: 0.5,
            pitch: 1.0,
            yaw: -0.1,
            thrust: 0.0,
        };
        let mut buf = [0; 255];
        encode(&msg, &mut buf).unwrap();
        let msg_decoded = decode(&buf).unwrap();
        assert_eq!(msg, msg_decoded)
    }

    #[test]
    fn encode_decode_motor_debug() {
        let msg = Message::MotorDebug {
            thrust: [0.1, 0.2, 0.3, 0.4],
        };
        let mut buf = [0; 255];
        encode(&msg, &mut buf).unwrap();
        let msg_decoded = decode(&buf).unwrap();
        assert_eq!(msg, msg_decoded)
    }
}
