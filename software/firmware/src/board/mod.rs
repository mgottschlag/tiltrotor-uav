#[cfg(feature = "blackpill")]
pub use blackpill::*;
#[cfg(feature = "bluepill")]
pub use bluepill::*;
#[cfg(feature = "feather_nrf52840")]
pub use feather_nrf52840::*;

#[cfg(feature = "blackpill")]
mod blackpill;
#[cfg(feature = "bluepill")]
mod bluepill;
#[cfg(feature = "feather_nrf52840")]
mod feather_nrf52840;
