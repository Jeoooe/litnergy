use std::io::{Read, Write};
use std::net::TcpStream;
use log::{error, info};
use crate::handler;
use crate::dev;

const PROTOCOL_MAJOR: i16 = 1;
const PROTOCOL_MINOR: i16 = 8;
const PROTOCOL_NAME: &[u8] = b"Barrier";
const CLIENT_NAME: &[u8] = b"rust-client";

#[derive(Debug)]
pub struct DeskflowClient {
    stream: TcpStream,
    pub mouse: dev::Mouse,
    pub enter_sequence: u32,
}

impl DeskflowClient {
    /// 连接到服务器
    pub fn init(host: &str, port: u16) -> std::io::Result<Self> {
        // 先创建鼠标
        let mouse = dev::Mouse::new()?;
        // 连接服务器
        let addr = format!("{}:{}", host, port);
        // println!("连接到 {}...", addr);
        let stream = TcpStream::connect(addr)?;
        Ok( Self { 
            stream,
            mouse,
            enter_sequence: 0
        })
    }


    /// 读取一段字节流（4字节长度 + 数据）
    pub fn read_from_server(&mut self) -> std::io::Result<Vec<u8>> {
        // println!("Waiting for data ");
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf)?;
        // println!("BUF: {:?}", buf);
        Ok(buf)
    }

    /// 发送定长字符串（4字节长度 + 数据）
    // pub fn write_string(&mut self, s: &str) -> std::io::Result<()> {
    //     let len = s.len() as u32;
    //     self.stream.write_all(&len.to_be_bytes())?;
    //     self.stream.write_all(s.as_bytes())?;
    //     Ok(())
    // }

    /// 发送固定大小的二进制数据
    pub fn write_raw(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.stream.write_all(data)
    }

    pub fn write_vec(&mut self, msg: &mut Vec<u8>) -> std::io::Result<()> {
        let mut final_msg = (msg.len() as u32).to_be_bytes().to_vec();
        final_msg.append(msg);
        self.stream.write_all(&final_msg)
    }

    /// 执行握手流程
    pub fn handshake(&mut self, client_name: &str) -> std::io::Result<()> {
        // Step 1: 接收服务器 Hello 消息
        // println!("等待服务器 Hello 消息...");
        let mut _rubbish = [0u8; 4];
        let mut hello_buf = [0u8; 7 + 2 + 2]; // 7字节协议名 + 2字节主版本 + 2字节次版本
        self.stream.read_exact(&mut _rubbish)?;
        self.stream.read_exact(&mut hello_buf)?;

        let protocol_name = std::str::from_utf8(&hello_buf[0..7])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let server_major = i16::from_be_bytes([hello_buf[7], hello_buf[8]]);
        let server_minor = i16::from_be_bytes([hello_buf[9], hello_buf[10]]);

        info!("Receive Hello:");
        info!("  Protocol name: '{}'", protocol_name.trim_end_matches('\0'));
        info!("  Server version: {}.{}", server_major, server_minor);

        if server_major != PROTOCOL_MAJOR {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Incompatible protocol version",
            ));
        }

        // Step 3: 发送 HelloBack 消息
        // 格式: 7字节协议名 + 2字节主版本 + 2字节次版本 + 字符串(客户端名)
        // println!("发送 HelloBack 消息...");

        let mut hello_back: Vec<u8> = PROTOCOL_NAME.to_vec();
        let mut major = PROTOCOL_MAJOR.to_be_bytes().to_vec();
        let mut minor = PROTOCOL_MINOR.to_be_bytes().to_vec();
        let length = CLIENT_NAME.len() as u32;
        hello_back.append(&mut major);
        hello_back.append(&mut minor);
        hello_back.append(&mut length.to_be_bytes().to_vec());
        hello_back.append(&mut client_name.as_bytes().to_vec());

        // println!("发送: {:?}", hello_back);
        self.write_vec(&mut hello_back)?;

        info!("  Client version: {}.{}", PROTOCOL_MAJOR, PROTOCOL_MINOR);

        Ok(())
    }

    pub fn run_client(&mut self) -> std::io::Result<()> {
        loop {
            match self.read_from_server() {
                Ok(msg) => {
                    if let Err(e) = handler::handle_message(&msg, self) {
                        return Err(e);
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}
