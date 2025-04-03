#![no_std]

use defmt::info;
use libm::fabsf;

#[derive(Clone, Debug, defmt::Format)]
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

#[derive(Clone, Debug, Copy, defmt::Format)]
pub enum Direction {
    Forward(f32),  // [0.0..1.0]
    Backward(f32), // [0.0..1.0]
    Stop,
}

pub trait Type {
    fn translate(&self, cmd: &Command) -> [Direction; 4];
}

fn motor_dir(input: f32) -> Direction {
    match input {
        _ if input < 0.0 => Direction::Backward(input),
        _ if input > 0.0 => Direction::Forward(input),
        _ => Direction::Stop,
    }
}

pub struct Car {}

impl Car {
    pub fn new() -> Self {
        Car {}
    }
}

impl Type for Car {
    fn translate(&self, cmd: &Command) -> [Direction; 4] {
        let diff = fabsf(cmd.pitch) * cmd.roll;
        let motor_left = motor_dir(cmd.pitch - diff);
        let motor_right = motor_dir(cmd.pitch + diff);
        info!(
            "motor_left={}, motor_right={}, diff={}",
            motor_left, motor_right, diff
        );

        [motor_left, motor_right, Direction::Stop, Direction::Stop]
    }
}
