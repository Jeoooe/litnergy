use log::{error, info};
use simplelog::SimpleLogger;
use clap::Parser;

use crate::client::ScreenSize;

mod handler;
mod helper;
mod dev;
mod client;

const DEFAULT_PORT: u16 = 24800;

#[derive(Parser)]
#[command(name = "litnergy")]
#[command(version = "0.1")]
#[command(about = "A Deskflow client based on uinput.", long_about = None)]
struct Cli {
    #[arg(long)]
    server: String,
    /// Default: 24800
    #[arg(long)]
    port: Option<u16>,
    /// example 1920x1080
    #[arg(short, long)]
    resolution: String,
    /// Client name, default: "litnergy"
    #[arg(short, long)]
    client_name: Option<String>,
}

fn parse_resolution(arg: String) -> std::io::Result<ScreenSize> {
    let resolution: Vec<&str> = arg.split("x").collect();
    let Ok(x) = resolution[0].parse::<u16>() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Wrong resolution format"));
    };
    let Ok(y) = resolution[1].parse::<u16>() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Wrong resolution format"));
    };
    Ok(ScreenSize { x, y })
}

fn main() {
    if cfg!(debug_assertions) {
        let _ = SimpleLogger::init(log::LevelFilter::Trace, simplelog::Config::default());
    } else {
        let _ = SimpleLogger::init(log::LevelFilter::Info, simplelog::Config::default());
    }

    let cli = Cli::parse();

    // SimpleLogger::new(log_level, config)
    // 配置
    let server_host = cli.server;
    let client_name = match cli.client_name {
        Some(s) => s,
        None => String::from("litnergy"),
    };
    let port = match cli.port {
        Some(port) => port,
        None => DEFAULT_PORT,
    };
    let resolution = parse_resolution(cli.resolution);
    let Ok(resolution) = resolution  else {
        error!("Wrong commandline argument: {}", resolution.unwrap_err());
        return;
    };

    info!("Client name: {}", client_name);
    info!("Connecting to: {}:{}", server_host, port);

    
    let res = client::DeskflowClient::init(resolution, &server_host, port);
    let Ok(mut client) = res else {
        error!("Init fail:{}", res.unwrap_err());
        return;
    };

    if let Err(e) = client.handshake(&client_name) {
        error!("\n Handshake fail: {}", e);
    }

    if let Err(e) = client.run_client() {
        error!("Runtime error:{}", e);
    }
}
