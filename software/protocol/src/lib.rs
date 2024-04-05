#![no_std]

use defmt::Format;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Status {
    pub r: f32, // roll
    pub p: f32, // pitch
    pub b: f32, // battery
}

impl Status {
    pub fn new() -> Self {
        Self {
            r: 0.0,
            p: 0.0,
            b: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Format, Clone)]
pub struct Command {
    // [0..255]
    pub thrust: [i16; 4],
    // [-1.0..1.0]
    pub pose: [f32; 2],
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
