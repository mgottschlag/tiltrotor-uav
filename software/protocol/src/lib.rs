#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    pub r: f32,
    pub p: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub thrust: [u8; 4],
}
