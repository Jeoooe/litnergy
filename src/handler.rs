// use std::net::TcpStream;
use log::{trace, warn};

use crate::{
    client::DeskflowClient,
    clipboard,
    dev::{ButtonType, KeyState},
    helper,
};

// const CNOP: &[u8] = b"\x00\x00\x00\x04CNOP";
const CALV_CODE: &[u8] = b"\x00\x00\x00\x04CALV";

pub fn handle_message(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    // trace!("收到消息: {:?}", msg);
    match &msg[..4] {
        b"QINF" => handle_qinf(msg, client),
        b"CIAK" => Ok(()),
        b"LSYN" | b"CROP" | b"DSOP" => Ok(()), //全都未完成,
        b"CCLP" => client.clipboard.handle_clipboard_grab(msg),
        b"DCLP" => client.clipboard.handle_clipboard_message(msg),
        b"CALV" => handle_calv(msg, client),
        b"DMMV" => handle_dmmv(msg, client),
        b"CINN" => handle_cinn(msg, client),
        b"DMDN" => handle_dmdn(msg, client),
        b"DMUP" => handle_dmup(msg, client),
        b"DMWM" => handle_dmwm(msg, client),
        b"COUT" => handle_cout(msg, client),
        b"EUNK" => handle_eunk(msg, client),
        b"DKDL" => handle_dkdl(msg, client),
        b"DKUP" => handle_dkup(msg, client),
        b"DKRP" => Ok(()), // 重复按键由内核完成, 无需上报
        _ => {  
            let code = msg[..4].to_vec();
            if let Ok(code) = String::from_utf8(code) {
                trace!("未编码消息: {}", code);
            }
            trace!("原始消息: {:?}", msg);
            
            Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "未知信息",
            ))
        },
    }
}


fn decode_mouse_button(code: u8) -> ButtonType {
    // * **Button IDs**:
    // * - `1`: Left button
    // * - `2`: Right button
    // * - `3`: Middle button
    // * - `4+`: Additional buttons (side buttons, etc.)
    // 这个好像是错的, 实际测试代码里的才应该是对的
    match code {
        1 => ButtonType::Left,
        2 => ButtonType::Middle,
        3 => ButtonType::Right,
        _ => ButtonType::Extra,
    } 
}

// Mouse up
fn handle_dmup(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    client.fake_device.button(decode_mouse_button(msg[4]),KeyState::UP)?;
    Ok(())
}

// Mouse Press
fn handle_dmdn(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
        client.fake_device.button(decode_mouse_button(msg[4]),KeyState::DOWN)?;
    Ok(())
}

// Mouse whell
fn handle_dmwm(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut code = [0u8; 2];
    code.clone_from_slice(&msg[4..6]);
    let horizon = i16::from_be_bytes(code);
    code.clone_from_slice(&msg[6..8]);
    let vertical = i16::from_be_bytes(code);
    client.fake_device.scroll(horizon, vertical)
}

// Absolute Mouse Movement
fn handle_dmmv(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    // * **Format**: `"DMMV%2i%2i"`
    let mut code: [u8; 2] = [0, 0];
    code.clone_from_slice(&msg[4..6]);
    let abs_x = i16::from_be_bytes(code);
    code.clone_from_slice(&msg[6..8]);
    let abs_y = i16::from_be_bytes(code);
    // trace!("X: {}, Y: {}", abs_x, abs_y);
    client.fake_device.move_abs(abs_x as i32, abs_y as i32)
}


// Keyboards
fn handle_dkdl(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut code = [0u8; 2];
    code.copy_from_slice(&msg[8..10]);
    let keycode = u16::from_be_bytes(code);
    client.fake_device.keyboard(keycode, KeyState::DOWN)
}

fn handle_dkup(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut code = [0u8; 2];
    code.copy_from_slice(&msg[8..10]);
    let keycode = u16::from_be_bytes(code);
    client.fake_device.keyboard(keycode, KeyState::UP)
}

fn handle_qinf(_msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    //**Format**: `"DINF%2i%2i%2i%2i%2i%2i%2i"`
    let mut msg = b"DINF".to_vec();
    let edge_cor = helper::get_edge_coordinate();
    let screen_size = client.screen_size;
    let warp = helper::get_warp_zone_size();
    let mouse = helper::get_mouse_position();
    let mut info = [
        edge_cor.0.to_be_bytes(),
        edge_cor.1.to_be_bytes(),
        screen_size.x.to_be_bytes(),
        screen_size.y.to_be_bytes(),
        warp.to_be_bytes(),
        mouse.0.to_be_bytes(),
        mouse.1.to_be_bytes(),
    ]
    .concat();
    msg.append(&mut info);
    trace!("发送DINF: {:?}", msg);
    client.write_vec(&mut msg)
}

// 保持活跃
fn handle_calv(_msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    client.write_raw(CALV_CODE)
}

// Enter Screen
fn handle_cinn(msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut code = [0u8; 2];
    code.clone_from_slice(&msg[4..6]);
    let abs_x = i16::from_be_bytes(code);
    code.clone_from_slice(&msg[6..8]);
    let abs_y = i16::from_be_bytes(code);
    // 我习惯上不去同步状态键
    // code.clone_from_slice(&msg[12..14]);
    // let key_mask = u16::from_be_bytes(code);
    let mut code = [0u8;4];
    code.clone_from_slice(&msg[8..12]);
    let sequence = u32::from_be_bytes(code);

    client.fake_device.move_abs(abs_x as i32, abs_y as i32)?;
    client.enter_sequence = sequence;
    // TODO: KeyMask
    // Waiting for implementation of keyboard
    Ok(())
}

// Leave Screen
fn handle_cout(_msg: &[u8], client: &mut DeskflowClient) -> std::io::Result<()> {
    // if let Err(e) = clipboard::send_local_payload(client) {
    //     warn!("Failed to send clipboard: {}", e);
    // }
    client.fake_device.leave_screen()
}

fn handle_eunk(_msg: &[u8], _client: &mut DeskflowClient) -> std::io::Result<()> {
    warn!("Unknown client name for server.");
    warn!("Please check spelling, server config, server host, or network.");
    warn!("Note: Server does not auto‑accept new clients – add this client manually first.");
    Err(std::io::Error::new(
    std::io::ErrorKind::Other,
    "Unknown client name",
    ))
}
