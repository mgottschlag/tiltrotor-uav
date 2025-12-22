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
        m1: f32, // [0.0 .. 1.0]
        m2: f32, // [0.0 .. 1.0]
        m3: f32, // [0.0 .. 1.0]
        m4: f32, // [0.0 .. 1.0]
    },
}

pub fn encode<'a>(msg: &Message, buf: &'a mut [u8]) -> Result<&'a mut [u8], postcard::Error> {
    postcard::to_slice(msg, buf)
}

pub fn decode(data: &[u8]) -> Result<Message, postcard::Error> {
    postcard::from_bytes(data)
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
        let data = encode(&msg, &mut buf).unwrap();
        let msg_decoded = decode(data).unwrap();
        assert_eq!(msg, msg_decoded)
    }

    #[test]
    fn encode_decode_motor_debug() {
        let msg = Message::MotorDebug {
            m1: 0.1,
            m2: 0.2,
            m3: 0.3,
            m4: 0.4,
        };
        let mut buf = [0; 255];
        let data = encode(&msg, &mut buf).unwrap();
        let msg_decoded = decode(data).unwrap();
        assert_eq!(msg, msg_decoded)
    }
}
