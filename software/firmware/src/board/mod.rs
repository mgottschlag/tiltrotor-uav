use defmt::Format;

#[cfg(feature = "blackpill")]
pub use blackpill::*;
#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "blackpill")]
mod blackpill;
#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, motor_left: Direction, motor_right: Direction);
}

/*pub trait PidTimer {
    fn elapsed_secs(&mut self) -> f32;
}*/

#[derive(Format)]
pub enum Direction {
    Forward(f32),
    Backward(f32),
    Stop,
}
