use embedded_hal::blocking::delay::DelayMs;
use mpu9250::{Mpu9250, SpiDevice};

pub use crate::board::{ImuCs, ImuIrq, ImuSpi};

pub struct Imu {
    mpu: mpu9250::Mpu9250<SpiDevice<ImuSpi, ImuCs>, Imu>,
}

impl Imu {
    pub fn new(spi: ImuSpi, cs: ImuCs, _irq: ImuIrq, delay: DelayMs<u8>) -> Self {
        let mut mpu = Mpu9250::marg_default(spi, cs, delay).unwrap();
        Imu { mpu }
    }
}
