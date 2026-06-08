// use std::net::TcpStream;

use log::trace;

use crate::{
    DeskflowClient, helper,
};

// const CNOP: &[u8] = b"\x00\x00\x00\x04CNOP";
const CALV_CODE: &[u8] = b"\x00\x00\x00\x04CALV";

pub fn handle_message(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    // trace!("收到消息: {:?}", msg);
    match &msg[..4] {
        b"QINF" => handle_qinf(msg, client),
        b"CIAK" => Ok(()),
        b"LSYN" | b"CROP" | b"DSOP" => Ok(()), //全都未完成, 
        b"CALV" => handle_calv(msg, client),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "接收到未知的信息",
        )),
    }
}

fn handle_calv(_msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    client.write_raw(CALV_CODE)
}

fn handle_qinf(_msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
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
    trace!("发送DINF: {:?}", msg);
    client.write_vec(&mut msg)
}
