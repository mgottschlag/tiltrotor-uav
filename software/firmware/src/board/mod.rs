#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, cmd: &Command);
}

#[derive(Clone, PartialEq, defmt::Format)]
pub enum Command {
    Remote {
        roll: f32,   // [-1.0 .. 1.0]
        pitch: f32,  // [-1.0 .. 1.0]
        yaw: f32,    // [-1.0 .. 1.0]
        thrust: f32, // [-1.0 .. 1.0]
    },
    MotorDebug {
        m1: f32, // [0.0 .. 1.0]
        m2: f32, // [0.0 .. 1.0]
        m3: f32, // [0.0 .. 1.0]
        m4: f32, // [0.0 .. 1.0]
    },
}
