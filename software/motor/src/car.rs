use libm::fabsf;

use crate::Command;
use crate::Direction;
use crate::Type;

const WHEEL_DISTANCE: f32 = 0.3;
const DEADZONE: f32 = 0.1;

pub struct Car {}

impl Car {
    pub fn new() -> Self {
        Car {}
    }
}

impl Type for Car {
    fn update(&self, cmd: &Command) -> [Direction; 4] {
        let roll = if fabsf(cmd.roll) >= DEADZONE {
            cmd.roll
        } else {
            0.0
        };
        let pitch = if fabsf(cmd.pitch) >= DEADZONE {
            cmd.pitch
        } else {
            0.0
        };

        let diff = roll * WHEEL_DISTANCE;
        let motor_left = motor_dir(pitch + diff);
        let motor_right = motor_dir(pitch - diff);

        [motor_left, motor_right, Direction::Stop, Direction::Stop]
    }
}

fn motor_dir(input: f32) -> Direction {
    match input {
        _ if input < 0.0 => Direction::Backward(fabsf(input).clamp(0.0_f32, 1.0_f32)),
        _ if input > 0.0 => Direction::Forward(input.clamp(0.0_f32, 1.0_f32)),
        _ => Direction::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let car = Car::new();

        // stop
        let motors = car.update(&Command {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(motors, [Direction::Stop; 4]);

        // forward
        let motors = car.update(&Command {
            roll: 0.0,
            pitch: 1.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(1.0),
                Direction::Forward(1.0),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // backward
        let motors = car.update(&Command {
            roll: 0.0,
            pitch: -1.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Backward(1.0),
                Direction::Backward(1.0),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // rotate left
        let motors = car.update(&Command {
            roll: 1.0,
            pitch: 0.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(WHEEL_DISTANCE),
                Direction::Backward(WHEEL_DISTANCE),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // rotate right
        let motors = car.update(&Command {
            roll: -1.0,
            pitch: 0.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Backward(WHEEL_DISTANCE),
                Direction::Forward(WHEEL_DISTANCE),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // slowly left
        let motors = car.update(&Command {
            roll: 1.0,
            pitch: 0.1,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(0.4),
                Direction::Backward(0.20000002),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // slowly right
        let motors = car.update(&Command {
            roll: -1.0,
            pitch: 0.1,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Backward(0.20000002),
                Direction::Forward(0.4),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // slightly left
        let motors = car.update(&Command {
            roll: 0.5,
            pitch: 0.5,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(0.65),
                Direction::Forward(0.35),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // slightly right
        let motors = car.update(&Command {
            roll: -0.5,
            pitch: 0.5,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(0.35),
                Direction::Forward(0.65),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // fully left
        let motors = car.update(&Command {
            roll: 1.0,
            pitch: 1.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(1.0),
                Direction::Forward(0.7),
                Direction::Stop,
                Direction::Stop
            ]
        );

        // fully right
        let motors = car.update(&Command {
            roll: -1.0,
            pitch: 1.0,
            yaw: 0.0,
            thrust: 0.0,
        });
        assert_eq!(
            motors,
            [
                Direction::Forward(0.7),
                Direction::Forward(1.0),
                Direction::Stop,
                Direction::Stop
            ]
        );
    }
}
