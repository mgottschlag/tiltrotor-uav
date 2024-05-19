use defmt::Format;

#[cfg(feature = "blackpill")]
pub use blackpill::*;
#[cfg(feature = "bluepill")]
pub use bluepill::*;
#[cfg(feature = "feather_nrf52840")]
pub use feather_nrf52840::*;
#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "blackpill")]
mod blackpill;
#[cfg(feature = "bluepill")]
mod bluepill;
#[cfg(feature = "feather_nrf52840")]
mod feather_nrf52840;
#[cfg(feature = "flightcontroller")]
mod flightcontroller;

use protocol::Command;

pub trait EnginePwm {
    fn update(&mut self, motor_left: Direction, motor_right: Direction);
}

pub trait RadioInterrupt {
    fn activate(&mut self);
    fn reset(&mut self);
}

pub trait PidTimer {
    fn elapsed_secs(&mut self) -> f32;
}

#[derive(Format)]
pub enum Direction {
    Forward(f32),
    Backward(f32),
    Stop,
}
