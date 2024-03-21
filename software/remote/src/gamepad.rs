use gilrs::{Button, Event, EventType, Gilrs};
use protocol::Command;
use std::{thread, time};

const NEW_COMMAND_DIFF: i16 = 10;

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

        loop {
            let mut new_cmd = cmd.clone();
            while let Some(event) = self.gilrs.next_event() {
                match event {
                    Event {
                        id: _,
                        event: EventType::ButtonChanged(Button::LeftTrigger2, value, _),
                        ..
                    } => {
                        new_cmd.thrust[0] = (255 as f32 * value) as i16;
                    }
                    Event {
                        id: _,
                        event: EventType::ButtonChanged(Button::RightTrigger2, value, _),
                        ..
                    } => {
                        new_cmd.thrust[1] = (255 as f32 * value) as i16;
                    }
                    _ => {}
                };
            }
            if thrust_changed(&new_cmd, &cmd) {
                cmd_tx.blocking_send(cmd).unwrap();
                cmd = new_cmd;
            }

            thread::sleep(time::Duration::from_millis(10));
        }
    }
}

fn thrust_changed(new_cmd: &Command, old_cmd: &Command) -> bool {
    !new_cmd
        .thrust
        .iter()
        .zip(old_cmd.thrust.iter())
        .all(|(new, old)| (new - old).abs() <= NEW_COMMAND_DIFF)
}

#[cfg(test)]
mod tests {

    use super::*;

    macro_rules! test_positive {
        ($name:ident, $( $new_cmd:expr, $old_cmd:expr ),* ) => {
            #[test]
            fn $name() {
                $(
                    assert!(thrust_changed(&$new_cmd, &$old_cmd));
                )*

            }
        };
    }

    macro_rules! test_negative {
        ($name:ident, $( $new_cmd:expr, $old_cmd:expr ),* ) => {
            #[test]
            fn $name() {
                $(
                    assert!(!thrust_changed(&$new_cmd, &$old_cmd));
                )*

            }
        };
    }

    test_positive!(
        test_positive_upper,
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        },
        Command {
            thrust: [61; 4],
            pose: [50, 2],
        }
    );
    test_positive!(
        test_positive_lower,
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        },
        Command {
            thrust: [39; 4],
            pose: [50, 2],
        }
    );
    test_positive!(
        test_positive_one_upper,
        Command {
            thrust: [50, 50, 50, 50],
            pose: [50, 2],
        },
        Command {
            thrust: [50, 50, 61, 50],
            pose: [50, 2],
        }
    );
    test_positive!(
        test_positive_one_lower,
        Command {
            thrust: [50, 50, 50, 50],
            pose: [50, 2],
        },
        Command {
            thrust: [50, 50, 39, 50],
            pose: [50, 2],
        }
    );

    test_negative!(
        test_negative_equals,
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        },
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        }
    );
    test_negative!(
        test_negative_upper,
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        },
        Command {
            thrust: [60; 4],
            pose: [50, 2],
        }
    );
    test_negative!(
        test_negative_lower,
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        },
        Command {
            thrust: [40; 4],
            pose: [50, 2],
        }
    );
    test_negative!(
        test_negative_one_upper,
        Command {
            thrust: [50, 60, 50, 50],
            pose: [50, 2],
        },
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        }
    );
    test_negative!(
        test_negative_one_lower,
        Command {
            thrust: [40, 50, 50, 50],
            pose: [50, 2],
        },
        Command {
            thrust: [50; 4],
            pose: [50, 2],
        }
    );
}
