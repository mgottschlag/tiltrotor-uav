use motor::Command;
use motor::Direction;

#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, cmd: &Command);
}

/*pub trait PidTimer {
    fn elapsed_secs(&mut self) -> f32;
}*/
