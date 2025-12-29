use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use futures_util::StreamExt;
use protocol::Message;
use protocol::encode;
use rustyline::error::ReadlineError;
use tokio::io::AsyncWriteExt;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::FramedRead;

mod decoder;

use decoder::FrameDecoder;

const HISTORY_FILE_NAME: &str = "history.txt";
const PROMPT: &str = "\x1b[1;33mUAV REMOTE \x1b[1;34m❯❯ \x1b[0m";

#[derive(Parser, Debug)]
struct Config {
    #[arg(short, long)]
    port: String,

    #[arg(short, long, default_value = "115200")]
    baud: u32,
}

fn parse_motor_array(s: &str) -> Result<[f32; 4]> {
    let res: Vec<f32> = s
        .split(',')
        .map(|v| {
            v.trim()
                .parse::<f32>()
                .expect(format!("Failed to parse '{v}' as float").as_str())
        })
        .collect();

    res.try_into()
        .map_err(|_| anyhow!("Expected 4 floats as input"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    let mut rl = rustyline::DefaultEditor::new()?;
    if let Err(e) = rl.load_history(HISTORY_FILE_NAME) {
        eprintln!("Failed to load history file: {e}");
    }

    let port = tokio_serial::new(config.port, config.baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()?;
    let (reader, mut writer) = tokio::io::split(port);
    let mut decoder = FramedRead::new(reader, FrameDecoder {});
    tokio::spawn(async move {
        while let Some(line) = decoder.next().await {
            let line = line.unwrap(); // TODO
            println!("???? {:?} (len={})", line, line.len());
            for b in &line {
                print!("{:02X} ", b);
            }
            println!();
            let msg = protocol::decode(&line).unwrap();
            println!("Foo: {:?}", msg);
        }
    });

    loop {
        let line = match rl.readline(PROMPT) {
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Failed to read input line: {e}");
                continue;
            }
            Ok(line) => {
                if let Err(e) = rl.add_history_entry(&line) {
                    eprintln!("Failed to store history entry: {e}");
                }
                line
            }
        };
        let args = match shlex::split(&line) {
            Some(args) => args,
            None => {
                eprintln!("Failed to parse arguments");
                continue;
            }
        };
        if args.len() > 0 {
            match args[0].as_str() {
                "exit" => break,
                "motors" => {
                    let thrust = parse_motor_array(&args[1])?;
                    let cmd = Message::MotorDebug { thrust: thrust };
                    let mut buf: [u8; 32] = [0; 32];
                    let data = encode(&cmd, &mut buf)?;
                    writer.write_all(data).await?;
                    writer.flush().await?;
                }
                _ => {}
            }
        }
    }

    if let Err(e) = rl.save_history(HISTORY_FILE_NAME) {
        eprintln!("Failed to save history file: {e}");
    }

    Ok(())
}
