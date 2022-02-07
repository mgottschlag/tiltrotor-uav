use mpu9250::{Marg, MargMeasurements, Mpu9250, SpiDevice};
use rtt_target::rprintln;

pub use crate::board::{ImuCs, ImuDelay, ImuIrq, ImuSpi};

pub struct Imu {
    mpu: mpu9250::Mpu9250<SpiDevice<ImuSpi, ImuCs>, Marg>,
}

impl Imu {
    pub fn new(spi: ImuSpi, cs: ImuCs, _irq: ImuIrq, delay: &mut ImuDelay) -> Self {
        rprintln!("IMU 1");
        let mut mpu = Mpu9250::marg_default(spi, cs, delay).unwrap();
        rprintln!("IMU 2");
        let all: MargMeasurements<[f32; 3]> = mpu.all().unwrap();
        rprintln!("{:?}", all);
        Imu { mpu }
    }
}
