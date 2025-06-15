use defmt::info;
use embassy_time::Delay;
use libm::{atan2f, powf, sqrtf};

pub use crate::board::{ImuCs, ImuSpi};

pub struct Rotations {
    pub pitch: f32,
    pub roll: f32,
    pub yaw: f32,
}

pub trait Driver {
    fn init(spi: ImuSpi, cs: ImuCs) -> Self;
    fn get_rotations(&mut self) -> Rotations;
}

pub struct Imu<D: Driver> {
    driver: D,
}

impl<D> Imu<D>
where
    D: Driver,
{
    pub fn init(driver: D) -> Self {
        Self { driver }
    }

    pub fn get_rotations(&mut self) -> Rotations {
        self.driver.get_rotations()
    }
}

pub struct Icm20689 {
    driver: icm20689::ICM20689<icm20689::SpiInterface<ImuSpi, ImuCs>>,
}

impl Driver for Icm20689 {
    fn init(spi: ImuSpi, cs: ImuCs) -> Self {
        let mut delay = Delay;
        let mut driver = icm20689::Builder::new_spi(spi, cs);
        info!(
            "Check device, device support = {}",
            driver.check_identity(&mut delay).unwrap()
        );
        driver.setup(&mut delay).unwrap();
        driver
            .set_accel_range(icm20689::AccelRange::Range_2g)
            .unwrap();
        driver
            .set_gyro_range(icm20689::GyroRange::Range_250dps)
            .unwrap();

        Self { driver }
    }

    fn get_rotations(&mut self) -> Rotations {
        let accel = self.driver.get_scaled_accel().unwrap();
        let _gyro = self.driver.get_scaled_gyro().unwrap();

        calc_rotations(accel)
    }
}

pub struct Mpu9250 {
    driver: mpu9250::Mpu9250<mpu9250::SpiDevice<ImuSpi, ImuCs>, mpu9250::Imu>,
}

impl Driver for Mpu9250 {
    fn init(spi: ImuSpi, cs: ImuCs) -> Self {
        let driver = mpu9250::Mpu9250::imu_default(spi, cs, &mut Delay {}).unwrap();
        Self { driver }
    }

    fn get_rotations(&mut self) -> Rotations {
        let accel: [f32; 3] = self.driver.accel().unwrap();
        calc_rotations(accel)
    }
}

fn calc_rotations(accel: [f32; 3]) -> Rotations {
    Rotations {
        pitch: atan2f(-accel[0], sqrtf(powf(accel[1], 2.) + powf(accel[2], 2.))) * 180.0 / 3.14159,
        roll: atan2f(accel[1], sqrtf(powf(accel[0], 2.) + powf(accel[2], 2.))) * 180.0 / 3.14159
            * (-1.0),
        yaw: 0.0,
    }
}
