use defmt::info;
use embassy_time::Delay;

pub use crate::board::{ImuCs, ImuSpi};

pub trait Driver {
    fn init(spi: ImuSpi, cs: ImuCs) -> Self;
    fn get_rotations(&mut self) -> ([f32; 3], [f32; 3]);
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

    pub fn get_rotations(&mut self) -> ([f32; 3], [f32; 3]) {
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

    fn get_rotations(&mut self) -> ([f32; 3], [f32; 3]) {
        let accel = self.driver.get_scaled_accel().unwrap();
        let gyro = self.driver.get_scaled_gyro().unwrap();
        (gyro, accel)
    }
}
