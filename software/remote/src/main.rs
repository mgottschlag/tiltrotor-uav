use bevy::prelude::*;
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::sync::mpsc;

mod radio;

const SCALE_POSE: f32 = 0.3;

#[derive(Component)]
struct Client {
    cmd_queue: mpsc::Sender<protocol::Command>,
}

#[derive(Resource)]
struct Status {
    status: protocol::Status,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opts {
    #[structopt(short = "d", long, parse(from_os_str), default_value = "/dev/ttyACM0")]
    device: PathBuf,

    #[structopt(short = "o", long)]
    offline: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TokioTasksPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, keyboard_input)
        .add_systems(Update, (bevy::window::close_on_esc, update_text))
        .run();
}

fn setup(
    runtime: ResMut<TokioTasksRuntime>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let opts = Opts::from_args();
    info!("Opts: {opts:?}");

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<protocol::Command>(32);
    let (status_tx, mut status_rx) = mpsc::channel::<protocol::Status>(32);
    commands.spawn(Camera2dBundle::default());
    commands.spawn(Client { cmd_queue: cmd_tx });

    let font = asset_server.load("FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::WHITE,
    };
    let text_justification = JustifyText::Center;
    commands.spawn((Text2dBundle {
        text: Text::from_section("Battery", text_style.clone()).with_justify(text_justification),
        ..default()
    },));

    match opts.offline {
        false => runtime.spawn_background_task(|_| async move {
            let mut radio = radio::Radio::new(opts.device, cmd_rx, status_tx)
                .await
                .unwrap();
            radio.run().await
        }),
        true => runtime.spawn_background_task(|_| async move {
            let mut last_cmd = protocol::Command::new();
            loop {
                if let Some(cmd) = cmd_rx.recv().await {
                    if cmd != last_cmd {
                        info!("Got {cmd:?}");
                        last_cmd = cmd;
                    }
                }
            }
        }),
    };

    commands.insert_resource(Status {
        status: protocol::Status::new(),
    });
    runtime.spawn_background_task(|mut ctx| async move {
        loop {
            let status = status_rx.recv().await.unwrap();
            info!("Status update: {status:?}");
            ctx.run_on_main_thread(move |ctx| {
                let mut res = ctx.world.resource_mut::<Status>();
                res.status = status;
            })
            .await;
        }
    });
}

fn keyboard_input(keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<&Client>) {
    let client = query.single_mut();

    let mut pressed = [false; 4]; // up, down, left, right
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        pressed[0] = true
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        pressed[1] = true
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        pressed[2] = true
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        pressed[3] = true
    }

    let cmd = match pressed {
        [true, false, false, false] => protocol::Command::new().with_pose([1.0 * SCALE_POSE, 0.0]),
        [false, true, false, false] => protocol::Command::new().with_pose([-1.0 * SCALE_POSE, 0.0]),
        [false, false, true, false] => protocol::Command::new().with_pose([0.0, 1.0 * SCALE_POSE]),
        [false, false, false, true] => protocol::Command::new().with_pose([0.0, -1.0 * SCALE_POSE]),

        [true, false, true, false] => {
            protocol::Command::new().with_pose([1.0 * SCALE_POSE, 1.0 * SCALE_POSE])
        }
        [true, false, false, true] => {
            protocol::Command::new().with_pose([1.0 * SCALE_POSE, -1.0 * SCALE_POSE])
        }
        [false, true, true, false] => {
            protocol::Command::new().with_pose([-1.0 * SCALE_POSE, 1.0 * SCALE_POSE])
        }
        [false, true, false, true] => {
            protocol::Command::new().with_pose([-1.0 * SCALE_POSE, -1.0 * SCALE_POSE])
        }

        _ => protocol::Command::new(),
    };

    futures::executor::block_on(async { client.cmd_queue.send(cmd).await }).unwrap();
}

fn update_text(status: Res<Status>, mut query: Query<&mut Text>) {
    for mut text in query.iter_mut() {
        text.sections[0].value = format!("Battery: {:.2} V", status.status.battery);
    }
}
