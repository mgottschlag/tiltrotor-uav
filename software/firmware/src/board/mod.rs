#[cfg(feature = "flightcontroller")]
pub use flightcontroller::*;

#[cfg(feature = "flightcontroller")]
mod flightcontroller;

pub trait EscDriver {
    fn update(&mut self, thrust: [f32; 4]);
}
