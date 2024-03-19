use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use protocol::Command;
use std::time::{Duration, SystemTime};

const KEY_ARROW_ENTER: Event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
const KEY_ARROW_ESC: Event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
const KEY_ARROW_CTRL_C: Event =
    Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
const KEY_ARROW_CTRL_D: Event =
    Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
const KEY_ARROW_W: Event = Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
const KEY_ARROW_S: Event = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
const KEY_ARROW_UP: Event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
const KEY_ARROW_DOWN: Event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
const KEY_ARROW_LEFT: Event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
const KEY_ARROW_RIGHT: Event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

pub async fn run(cmd_tx: &tokio::sync::mpsc::Sender<Command>) {
    let mut reader = EventStream::new();

    let mut thrust: [i16; 4] = [0; 4];
    let mut pose: [i8; 2] = [0; 2];
    let mut last_event = SystemTime::now();
    let mut pressed = false;
    println!("Listening for packets:\r");

    loop {
        let delay = Delay::new(Duration::from_millis(10)).fuse();
        let event = reader.next().fuse();
        tokio::select! {
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        //println!("Event::{:?}\r", event);

                        // add newline to terminal
                        if event == KEY_ARROW_ENTER.into() {
                            println!("\r");
                        }

                        // stop remote
                        // - ESC
                        // - CTRL+C
                        // - CTRL+D
                        if event == KEY_ARROW_ESC.into() || event == KEY_ARROW_CTRL_C.into() || event == KEY_ARROW_CTRL_D.into() {
                            cmd_tx
            .send(Command {
                thrust: [0; 4],
                pose: [0; 2],
            })
            .await
            .unwrap();
                            break;
                        }

                        // control thrust
                        // - 'w' to go up
                        // - 's' to go down
                        if event == KEY_ARROW_W.into() {
                            thrust = thrust.map(|e| {e+10});
                            cmd_tx.send(Command {
                                thrust: thrust,
                                pose: pose,
                            }).await.unwrap();
                        }
                        if event == KEY_ARROW_S.into() {
                            thrust = thrust.map(|e| {e-10});
                            cmd_tx.send(Command {
                                thrust: thrust,
                                pose: pose,
                            }).await.unwrap();
                        }

                        // control pose via arrow keys
                        if event == KEY_ARROW_UP.into() {
                            last_event = SystemTime::now();
                            pressed = true;
                            pose[0] = 20;
                            cmd_tx.send(Command {
                                thrust: thrust,
                                pose: pose,
                            }).await.unwrap();
                        }
                        if event == KEY_ARROW_DOWN.into() {
                            last_event = SystemTime::now();
                                pressed = true;
                                pose[0] = -20;
                                cmd_tx.send(Command {
                                    thrust: thrust,
                                    pose: pose,
                                }).await.unwrap();
                        }
                        if event == KEY_ARROW_LEFT.into() {
                            last_event = SystemTime::now();
                            pressed = true;
                            pose[1] = 20;
                            cmd_tx.send(Command {
                                thrust: thrust,
                                pose: pose,
                            }).await.unwrap();
                        }
                        if event == KEY_ARROW_RIGHT.into() {
                            last_event = SystemTime::now();
                            pressed = true;
                            pose[1] = -20;
                            cmd_tx.send(Command {
                                thrust: thrust,
                                pose: pose,
                            }).await.unwrap();
                        }
                    }
                    Some(Err(e)) => println!("Error: {e:?}\r"),
                    None => break,
                }
            }
            _ = delay => {
                if pressed && last_event.elapsed().unwrap() > Duration::new(0, 500_000_000) {
                    pressed = false;
                    pose = [0; 2];
                    cmd_tx.send(Command {
                        thrust: thrust,
                        pose: pose,
                    }).await.unwrap();
                }
            },
        }
    }
}
