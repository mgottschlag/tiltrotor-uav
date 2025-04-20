use motor::Command;
use motor::Direction;

#[cfg(feature = "blackpill")]
pub use blackpill::*;

#[cfg(feature = "blackpill")]
mod blackpill;

pub trait EnginePwm {
    fn update(&mut self, cmd: &Command);
}

/*pub trait PidTimer {
    fn elapsed_secs(&mut self) -> f32;
}*/
