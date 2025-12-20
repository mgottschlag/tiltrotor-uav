#![no_std]

#[derive(PartialEq)]
pub enum Command {
    Remote {
        roll: f32,
        pitch: f32,
        yaw: f32,
        thrust: f32,
    },
    MotorDebug {
        m1: f32,
        m2: f32,
        m3: f32,
        m4: f32,
    },
}
