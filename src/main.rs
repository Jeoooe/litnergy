use log::{error, info};
use simplelog::SimpleLogger;

mod handler;
mod helper;
mod dev;
mod client;

// Deskflow 协议常量
const DEFAULT_PORT: u16 = 24800;

fn main() -> std::io::Result<()> {
    if cfg!(debug_assertions) {
        let _ = SimpleLogger::init(log::LevelFilter::Trace, simplelog::Config::default());
    } else {
        let _ = SimpleLogger::init(log::LevelFilter::Info, simplelog::Config::default());
    }

    // SimpleLogger::new(log_level, config)
    // 配置
    let server_host = "192.168.1.103"; // 修改为你的服务器地址
    let client_name = "rust-client";

    info!("目标服务器: {}:{}", server_host, DEFAULT_PORT);
    info!("客户端名称: {}", client_name);

    // 连接并握手
    let mut client = client::DeskflowClient::init(server_host, DEFAULT_PORT)?;

    match client.handshake(client_name) {
        Ok(()) => {
            // println!("\n 握手成功！");
            // println!("现在可以接收/发送消息了...");

            // 简单的事件循环，读取消息直到连接关闭
            // println!("按 Ctrl+C 退出\n");
            loop {
                if let Err(e) = client.read_message() {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("连接已关闭");
                        // println!("服务器断开连接");
                        break;
                    }
                    error!("读取消息错误: {}", e);
                    break;
                }
            }
        }
        Err(e) => {
            error!("\n 握手失败: {}", e);
        }
    }
    Ok(())
}
