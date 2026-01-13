#![no_std]

use fusion_ahrs::Ahrs;
use nalgebra::UnitQuaternion;
use nalgebra::Vector3;

pub struct Kf {
    ahrs: Ahrs,
    dt: f32,
}

impl Kf {
    pub fn new(dt: f32) -> Self {
        Self {
            ahrs: Ahrs::new(),
            dt,
        }
    }

    pub fn update(
        &mut self,
        gyro: [f32; 3],
        accel: [f32; 3],
        thrust: [f32; 4],
    ) -> ([f32; 2], [f32; 4]) {
        self.ahrs
            .update_no_magnetometer(Vector3::from(gyro), Vector3::from(accel), self.dt);
        let quat: UnitQuaternion<f32> = self.ahrs.quaternion();
        let (roll, pitch, _yaw) = quat.euler_angles();

        // Tuning values of PD controller:
        // (1) Start with low values (kp = 1..3, kd = 0.1..0.3).
        // (2) Increase kp until the quad responds fast enough but does not oscillate.
        // (3) Increase kd to damp oscillations / smooth response.
        // (4) Adjust hover base thrust to maintain altitude.
        let kp = 3.0;
        let kd = 0.2;
        let roll_rate = gyro[0];
        let pitch_rate = gyro[1];

        (
            [roll_rate, pitch_rate],
            [
                thrust[0] * 0.6 + kp * roll - kd * roll_rate,
                thrust[1] * 0.6 - kp * roll - kd * roll_rate,
                thrust[2] * 0.6 - kp * pitch - kd * pitch_rate,
                thrust[3] * 0.6 + kp * pitch - kd * pitch_rate,
            ],
        )
    }
}
