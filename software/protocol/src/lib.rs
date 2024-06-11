#![no_std]

use defmt::Format;
use minicbor::{CborLen, Decode, Encode};

#[derive(Debug, PartialEq, Clone, Encode, Decode, CborLen)]
pub struct Status {
    #[n(0)]
    pub roll: f32,
    #[n(1)]
    pub pitch: f32,
    #[n(2)]
    pub battery: f32,
}

impl Status {
    pub fn new() -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            battery: 0.0,
        }
    }
}

#[derive(Debug, PartialEq, Format, Clone, Encode, Decode, CborLen)]
pub struct Command {
    #[n(0)]
    pub thrust: [i16; 4], // [0..255]
    #[n(1)]
    pub pose: [f32; 2], // [-1.0..1.0]
}

impl Command {
    pub fn new() -> Self {
        Self {
            thrust: [0; 4],
            pose: [0.0; 2],
        }
    }

    pub fn with_thrust(&mut self, thrust: [i16; 4]) -> Self {
        Self {
            thrust: thrust,
            pose: self.pose,
        }
    }

    pub fn with_pose(&mut self, pose: [f32; 2]) -> Self {
        Self {
            thrust: self.thrust,
            pose: pose,
        }
    }

    pub fn scale_pose(&mut self, scale: f32) -> Self {
        Self {
            thrust: self.thrust,
            pose: self.pose.map(|e| e * scale),
        }
    }
}
