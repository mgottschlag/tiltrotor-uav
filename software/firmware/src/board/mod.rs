#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EnginePwm {
    fn update(&mut self, cmd: &protocol::Message);
}
