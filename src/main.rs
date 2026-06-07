use std::ffi::FromBytesWithNulError;
use std::io::{Read, Write};
use std::net::TcpStream;
mod handler;
mod helper;
mod protocol;

// Deskflow 协议常量
const DEFAULT_PORT: u16 = 24800;
const PROTOCOL_MAJOR: i16 = 1;
const PROTOCOL_MINOR: i16 = 8;
const PROTOCOL_NAME: &[u8] = b"Barrier";
const CLIENT_NAME: &[u8] = b"rust-client";

/// Deskflow 客户端握手实现
struct DeskflowClient {
    stream: TcpStream,
}

impl DeskflowClient {
    /// 连接到服务器
    fn connect(host: &str, port: u16) -> std::io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        println!("连接到 {}...", addr);
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }

    /// 读取一段字节流（4字节长度 + 数据）
    fn read_from_server(&mut self) -> std::io::Result<Vec<u8>> {
        println!("Waiting for data ");
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf)?;
        println!("BUF: {:?}", buf);
        Ok(buf)
    }

    /// 发送定长字符串（4字节长度 + 数据）
    fn write_string(&mut self, s: &str) -> std::io::Result<()> {
        let len = s.len() as u32;
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(s.as_bytes())?;
        Ok(())
    }

    /// 发送固定大小的二进制数据
    fn write_raw(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.stream.write_all(data)
    }

    fn write_vec(&mut self, msg: &mut Vec<u8>) -> std::io::Result<()> {
        let mut final_msg = (msg.len() as u32).to_be_bytes().to_vec();
        final_msg.append(msg);
        self.stream.write_all(&final_msg)
    }

    /// 执行握手流程
    fn handshake(&mut self, client_name: &str) -> std::io::Result<()> {
        // Step 1: 接收服务器 Hello 消息
        println!("等待服务器 Hello 消息...");
        let mut _rubbish = [0u8; 4];
        let mut hello_buf = [0u8; 7 + 2 + 2]; // 7字节协议名 + 2字节主版本 + 2字节次版本
        self.stream.read_exact(&mut _rubbish)?;
        self.stream.read_exact(&mut hello_buf)?;

        let protocol_name = std::str::from_utf8(&hello_buf[0..7])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let server_major = i16::from_be_bytes([hello_buf[7], hello_buf[8]]);
        let server_minor = i16::from_be_bytes([hello_buf[9], hello_buf[10]]);

        println!("收到服务器 Hello:");
        println!("  协议名: '{}'", protocol_name.trim_end_matches('\0'));
        println!("  服务器版本: {}.{}", server_major, server_minor);
        println!("原始数据 {:?}", hello_buf);

        // Step 2: 检查协议兼容性
        if server_major != PROTOCOL_MAJOR {
            println!(
                "协议版本不兼容！服务器版本: {}.{}",
                server_major, server_minor
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Incompatible protocol version",
            ));
        }

        // Step 3: 发送 HelloBack 消息
        // 格式: 7字节协议名 + 2字节主版本 + 2字节次版本 + 字符串(客户端名)
        println!("发送 HelloBack 消息...");

        // 固定部分: 协议名(7字节，不足补0) + 版本号
        let mut hello_back: Vec<u8> = PROTOCOL_NAME
            .iter()
            .chain(PROTOCOL_MAJOR.to_be_bytes().iter())
            .chain(PROTOCOL_MINOR.to_be_bytes().iter())
            .copied()
            .collect();
        let length = CLIENT_NAME.len();
        hello_back.append(&mut length.to_be_bytes().to_vec());
        hello_back.append(&mut CLIENT_NAME.to_vec());

        self.write_vec(&mut hello_back)?;
        println!("发送: {:?}", hello_back);

        println!("  客户端名: {}", client_name);
        println!("  客户端版本: {}.{}", PROTOCOL_MAJOR, PROTOCOL_MINOR);

        // Step 4: 等待可选的消息（如 QINF 查询）
        // 在实际握手后，服务器可能会发送 QINF 等消息
        println!("握手完成！等待后续消息...");

        Ok(())
    }

    /// 读取并显示一条消息（用于调试）
    fn read_message(&mut self) -> std::io::Result<()> {
        match self.read_from_server() {
            Ok(msg) => {
                println!("原始消息: {:?}", msg);
                let res = handler::handle_message(&msg, self);
                if res.is_err() {
                    println!("未编码消息: {:?}", msg);
                }
                // 可以在这里根据消息类型读取更多数据
                // 简单起见，这里只是跳过消息体
                // 实际应该解析消息格式
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("连接已关闭");
                Err(e)
            }
            Err(e) => Err(e),
        }
    }
}

fn main() -> std::io::Result<()> {
    // 配置
    let server_host = "192.168.1.104"; // 修改为你的服务器地址
    let client_name = "rust-client";

    println!("=== Deskflow 简单客户端握手示例 ===");
    println!("目标服务器: {}:{}", server_host, DEFAULT_PORT);
    println!("客户端名称: {}", client_name);
    println!();

    // 连接并握手
    let mut client = DeskflowClient::connect(server_host, DEFAULT_PORT)?;

    match client.handshake(client_name) {
        Ok(()) => {
            println!("\n 握手成功！");
            println!("现在可以接收/发送消息了...");

            // 简单的事件循环，读取消息直到连接关闭
            println!("按 Ctrl+C 退出\n");
            loop {
                if let Err(e) = client.read_message() {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        println!("服务器断开连接");
                        break;
                    }
                    println!("读取消息错误: {}", e);
                    break;
                }
            }
        }
        Err(e) => {
            println!("\n❌ 握手失败: {}", e);
        }
    }
    println!("客户端结束");
    Ok(())
}
