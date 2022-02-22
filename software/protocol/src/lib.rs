#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Status {
    pub r: f32,
    pub p: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Command {
    // [0..255]
    pub thrust: [u8; 4],
    // [-90..90]
    pub pose: [i8; 2],
}
