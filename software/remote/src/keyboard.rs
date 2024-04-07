use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use protocol::Command;
use std::time::{Duration, SystemTime};

const KEY_ESC: Event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
const KEY_CTRL_C: Event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
const KEY_CTRL_D: Event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
const KEY_UP: Event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
const KEY_DOWN: Event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
const KEY_LEFT: Event = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
const KEY_RIGHT: Event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

const SCALE_POSE: f32 = 0.2;

pub async fn run(cmd_tx: &tokio::sync::mpsc::Sender<Command>) {
    let mut reader = EventStream::new();
    println!("Listening for packets:\r");

    let mut last_event = SystemTime::now();
    let mut last_cmd = Command::new();
    loop {
        let delay = Delay::new(Duration::from_millis(10)).fuse();
        let event = reader.next().fuse();
        tokio::select! {
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        //println!("Event::{:?}\r", event);

                        // stop remote
                        // - ESC
                        // - CTRL+C
                        // - CTRL+D
                        if event == KEY_ESC.into() || event == KEY_CTRL_C.into() || event == KEY_CTRL_D.into() {
                            cmd_tx.send(Command::new()).await.unwrap();
                            break;
                        }

                        {
                            let mut cmd = last_cmd.clone();
                            last_event = SystemTime::now();

                            // control pose via arrow keys
                            if event == KEY_UP.into() {
                                cmd = Command::new().with_pose([1.0*SCALE_POSE, 0.0]);
                            }
                            if event == KEY_DOWN.into() {
                                cmd = Command::new().with_pose([-1.0*SCALE_POSE, 0.0]);
                            }
                            if event == KEY_LEFT.into() {
                                cmd = Command::new().with_pose([0.0*SCALE_POSE, -0.2]);
                            }
                            if event == KEY_RIGHT.into() {
                                cmd = Command::new().with_pose([0.0*SCALE_POSE, 0.2]);
                            }

                            if cmd != last_cmd {
                                cmd_tx.send(cmd.clone()).await.unwrap();
                                last_cmd = cmd;
                            }
                        }
                    }
                    Some(Err(e)) => println!("Error: {e:?}\r"),
                    None => break,
                }
            }
            _ = delay => {
                // There is no dedicated 'release' event.
                // Therefore, we have to check periodically if there is still any key pressed.
                if last_event.elapsed().unwrap() > Duration::new(0, 500_000_000) {
                    if last_cmd != Command::new() {
                        cmd_tx.send(Command::new()).await.unwrap();
                        last_cmd = Command::new();
                    }
                }
            },
        }
    }
}
