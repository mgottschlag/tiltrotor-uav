use libm::{atan2f, powf, sqrtf};
use mpu9250::{ImuMeasurements, Mpu9250, SpiDevice};
use rtt_target::rprintln;

pub use crate::board::{ImuCs, ImuDelay, ImuIrq, ImuSpi};

pub struct Imu {
    mpu: mpu9250::Mpu9250<SpiDevice<ImuSpi, ImuCs>, mpu9250::Imu>,
}

pub struct Rotations {
    pub pitch: f32,
    pub roll: f32,
    pub yaw: f32,
}

impl Imu {
    pub fn new(spi: ImuSpi, cs: ImuCs, _irq: ImuIrq, delay: &mut ImuDelay) -> Self {
        let mpu = Mpu9250::imu_default(spi, cs, delay).unwrap();
        Imu { mpu }
    }

    pub fn get_rotations(&mut self) -> Rotations {
        let accel: [f32; 3] = self.mpu.accel().unwrap();

        Rotations {
            pitch: atan2f(accel[1], sqrtf(powf(accel[0], 2.) + powf(accel[2], 2.))) * 180.0
                / 3.14159
                * (-1.0),
            roll: atan2f(-accel[0], sqrtf(powf(accel[1], 2.) + powf(accel[2], 2.))) * 180.0
                / 3.14159
                * (1.0),
            yaw: 0.0,
        }
    }
}
