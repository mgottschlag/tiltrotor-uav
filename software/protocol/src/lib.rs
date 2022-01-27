#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Status {
    pub r: f32,
    pub p: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Command {
    pub thrust: [u8; 4],
}
