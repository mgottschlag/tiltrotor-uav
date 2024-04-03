#![no_std]

use defmt::Format;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Status {
    pub r: f32,
    pub p: f32,
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
}
