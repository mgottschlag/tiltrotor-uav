#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, cmd: &Command);
}

#[derive(PartialEq, defmt::Format)]
pub enum Command {
    Remote {
        roll: f32,
        pitch: f32,
        yaw: f32,
        thrust: f32,
    },
    MotorDebug {
        m1: f32,
        m2: f32,
        m3: f32,
        m4: f32,
    },
}
