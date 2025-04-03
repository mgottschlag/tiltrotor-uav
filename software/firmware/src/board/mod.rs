use motor::Command;
use motor::Direction;

#[cfg(feature = "blackpill")]
pub use blackpill::*;
#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "blackpill")]
mod blackpill;
#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, cmd: &Command);
}

/*pub trait PidTimer {
    fn elapsed_secs(&mut self) -> f32;
}*/
