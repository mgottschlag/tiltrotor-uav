use gilrs::{Axis, Event, EventType, Gilrs};
use protocol::Command;
use std::{thread, time};

const DEADZONE: i8 = 15;
const MINIMAL_CHANGE_DIFF: i8 = 9;

pub struct Gamepad {
    gilrs: Gilrs,
}

impl Gamepad {
    pub fn init() -> Option<Self> {
        let gilrs = Gilrs::new().unwrap();
        let gamepads = gilrs.gamepads();

        let mut found = false;
        for (_id, gamepad) in gamepads {
            println!("- {}", gamepad.name());
            found = true;
        }

        match found {
            true => Some(Gamepad { gilrs }),
            false => None,
        }
    }

    pub fn run(&mut self, cmd_tx: &tokio::sync::mpsc::Sender<Command>) {
        let mut cmd = Command {
            thrust: [0; 4],
            pose: [0; 2],
        };

        let mut new_cmd = cmd.clone();
        loop {
            while let Some(event) = self.gilrs.next_event() {
                match event {
                    Event {
                        id: _,
                        event: EventType::AxisChanged(Axis::LeftStickY, value, _),
                        ..
                    } => new_cmd.pose[0] = (value * 90 as f32) as i8,
                    Event {
                        id: _,
                        event: EventType::AxisChanged(Axis::LeftStickX, value, _),
                        ..
                    } => new_cmd.pose[1] = (value * 90 as f32) as i8,
                    _ => {}
                };
            }
            new_cmd = ensure_deadzone(new_cmd, DEADZONE);
            if ensure_pose_changed(&new_cmd, &cmd, MINIMAL_CHANGE_DIFF) {
                cmd = new_cmd.clone();
                cmd_tx.blocking_send(cmd.clone()).unwrap();
            }

            thread::sleep(time::Duration::from_millis(1));
        }
    }
}

fn ensure_deadzone(mut cmd: Command, deadzone: i8) -> Command {
    cmd.pose.iter_mut().for_each(|value| {
        if (*value).abs() <= deadzone {
            *value = 0
        }
    });
    cmd
}

fn ensure_pose_changed(new_cmd: &Command, old_cmd: &Command, minimal_change_diff: i8) -> bool {
    !new_cmd
        .pose
        .iter()
        .zip(old_cmd.pose.iter())
        .all(|(new, old)| (*new - *old).abs() <= minimal_change_diff)
}

#[cfg(test)]
mod tests {

    use super::*;

    macro_rules! test_ensure_deadzone {
        ($name:ident, $( $cmd_in:expr, $cmd_out:expr ),* ) => {
            #[test]
            fn $name() {
                $(
                    let cmd = ensure_deadzone($cmd_in, DEADZONE);
                    assert!(cmd == $cmd_out);
                )*
            }
        };
    }

    test_ensure_deadzone!(
        test_ensure_deadzone_1,
        Command::new().with_pose([9, 9]),
        Command::new().with_pose([0, 0])
    );
    test_ensure_deadzone!(
        test_ensure_deadzone_2,
        Command::new().with_pose([0, 9]),
        Command::new().with_pose([0, 0])
    );
    test_ensure_deadzone!(
        test_ensure_deadzone_3,
        Command::new().with_pose([9, 0]),
        Command::new().with_pose([0, 0])
    );
    test_ensure_deadzone!(
        test_ensure_deadzone_4,
        Command::new().with_pose([0, 0]),
        Command::new().with_pose([0, 0])
    );
    test_ensure_deadzone!(
        test_ensure_deadzone_5,
        Command::new().with_pose([10, 9]),
        Command::new().with_pose([10, 0])
    );
    test_ensure_deadzone!(
        test_ensure_deadzone_6,
        Command::new().with_pose([9, 10]),
        Command::new().with_pose([0, 10])
    );

    macro_rules! test_ensure_pose_changed {
        ($name:ident, $( $new_cmd:expr, $old_cmd:expr, $res:expr ),* ) => {
            #[test]
            fn $name() {
                $(
                    match $res {
                        true => assert!(ensure_pose_changed(&$new_cmd, &$old_cmd, MINIMAL_CHANGE_DIFF)),
                        false => assert!(!ensure_pose_changed(&$new_cmd, &$old_cmd, MINIMAL_CHANGE_DIFF))
                    }
                )*

            }
        };
    }

    test_ensure_pose_changed!(
        test_ensure_pose_changed_1,
        Command::new().with_pose([50, 0]),
        Command::new().with_pose([59, 0]),
        false
    );
    test_ensure_pose_changed!(
        test_ensure_pose_changed_2,
        Command::new().with_pose([-50, 0]),
        Command::new().with_pose([-41, 0]),
        false
    );
    test_ensure_pose_changed!(
        test_ensure_pose_changed_3,
        Command::new().with_pose([0, 50]),
        Command::new().with_pose([0, 59]),
        false
    );
    test_ensure_pose_changed!(
        test_ensure_pose_changed_4,
        Command::new().with_pose([0, -50]),
        Command::new().with_pose([0, -41]),
        false
    );
    test_ensure_pose_changed!(
        test_ensure_pose_changed_5,
        Command::new().with_pose([-50, 50]),
        Command::new().with_pose([-59, 60]),
        true
    );
    test_ensure_pose_changed!(
        test_ensure_pose_changed_6,
        Command::new().with_pose([50, -50]),
        Command::new().with_pose([41, -40]),
        true
    );
}
