use anyhow::Result;
use clap::Parser;
use log::info;
use protocol::{Message, encode};
use tokio::io::AsyncWriteExt;
use tokio_serial::SerialPortBuilderExt;

#[derive(Parser, Debug)]
struct Config {
    #[arg(short, long)]
    port: String,

    #[arg(short, long, default_value = "115200")]
    baud: u32,

    #[arg(short, long, value_parser = parse_motor_array)]
    values: [f32; 4],
}

fn parse_motor_array(s: &str) -> Result<[f32; 4], String> {
    let res: Vec<f32> = s
        .split(',')
        .map(|v| {
            v.trim()
                .parse::<f32>()
                .expect(format!("Failed to parse '{v}' as float").as_str())
        })
        .collect();
    Ok(res
        .try_into()
        .expect(format!("Expected 4 floats as input").as_str()))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = Config::parse();
    info!("Config: {config:?}");

    let mut port = tokio_serial::new(config.port, config.baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()?;

    let cmd = Message::MotorDebug {
        m1: config.values[0],
        m2: config.values[1],
        m3: config.values[2],
        m4: config.values[3],
    };
    let mut buf: [u8; 32] = [0; 32];
    let data = encode(&cmd, &mut buf)?;
    info!("Sending message {cmd:?} (len={})", data.len());

    port.write_all(data).await?;
    port.flush().await?;

    Ok(())
}
