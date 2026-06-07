use std::net::TcpStream;

use crate::{
    DeskflowClient, helper,
    protocol::{
        self,
        ProtocolCode::{LSYN, QINF},
    },
};

const CNOP: &[u8] = b"\x00\x00\x00\x04CNOP";

pub fn handle_message(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    let code = protocol::message_to_code(msg);
    println!("收到消息: {:?}", code);
    match code {
        QINF => handle_QINF(msg, client),
        LSYN => Ok(()),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "接收到未知的信息",
        )),
    }
}

fn handle_QINF(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    //**Format**: `"DINF%2i%2i%2i%2i%2i%2i%2i"`
    let mut msg = b"DINF".to_vec();
    let edge_cor = helper::get_edge_coordinate();
    let screen_size = helper::get_screen_size();
    let warp = helper::get_warp_zone_size();
    let mouse = helper::get_mouse_position();
    let mut info = [
        edge_cor.0.to_be_bytes(),
        edge_cor.1.to_be_bytes(),
        screen_size.0.to_be_bytes(),
        screen_size.1.to_be_bytes(),
        warp.to_be_bytes(),
        mouse.0.to_be_bytes(),
        mouse.1.to_be_bytes(),
    ]
    .concat();
    msg.append(&mut info);
    println!("发送DINF: {:?}", msg);
    client.write_vec(&mut msg)
}
