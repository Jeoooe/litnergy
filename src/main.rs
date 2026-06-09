use log::{error, info};
use simplelog::SimpleLogger;

mod handler;
mod helper;
mod dev;
mod client;

// Deskflow 协议常量
const DEFAULT_PORT: u16 = 24800;

fn main() {
    if cfg!(debug_assertions) {
        let _ = SimpleLogger::init(log::LevelFilter::Trace, simplelog::Config::default());
    } else {
        let _ = SimpleLogger::init(log::LevelFilter::Info, simplelog::Config::default());
    }

    // SimpleLogger::new(log_level, config)
    // 配置
    let server_host = "192.168.1.103"; // 修改为你的服务器地址
    let client_name = "rust-client";

    info!("Client name: {}", client_name);
    info!("Connecting to: {}:{}", server_host, DEFAULT_PORT);

    
    let res = client::DeskflowClient::init(server_host, DEFAULT_PORT);
    let Ok(mut client) = res else {
        error!("Init fail:{}", res.unwrap_err());
        return;
    };

    if let Err(e) = client.handshake(client_name) {
        error!("\n Handshake fail: {}", e);
    }

    if let Err(e) = client.run_client() {
        error!("Runtime error:{}", e);
    }
}
