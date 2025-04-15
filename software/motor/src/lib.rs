#![no_std]

pub mod car;

#[derive(Clone, Debug, PartialEq, defmt::Format)]
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

#[derive(Clone, Debug, Copy, PartialEq, PartialOrd, defmt::Format)]
pub enum Direction {
    Forward(f32),  // [0.0..1.0]
    Backward(f32), // [0.0..1.0]
    Stop,
}

pub trait Type {
    fn update(&self, cmd: &Command) -> [Direction; 4];
}
