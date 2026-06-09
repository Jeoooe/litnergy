use evdev_rs::enums::EV_KEY;

pub struct EdgeCoordinate(pub i16, pub i16);
pub struct MousePosition(pub i16, pub i16);

pub const KEYCODE_LIMIT: u32 = 256;

pub fn get_edge_coordinate() -> EdgeCoordinate {
    // 临时
    EdgeCoordinate(0i16, 0i16)
}

/// ? 我不知道这是干嘛的
pub fn get_warp_zone_size() -> u16 {
    0
}

pub fn get_mouse_position() -> MousePosition {
    MousePosition(0i16, 0i16)
}

// Scancode -> Keycode
// 并不是完整的映射, windows到linux的扫描码不是一一对应的
// 特例情况就硬编码
pub fn scancode_to_keycode(scancode: u16) -> Option<EV_KEY> {
    match scancode {
        347 => Some(EV_KEY::KEY_LEFTMETA),    // windows键映射
        328 => Some(EV_KEY::KEY_UP),
        331 => Some(EV_KEY::KEY_LEFT),
        333 => Some(EV_KEY::KEY_RIGHT),
        336 => Some(EV_KEY::KEY_DOWN),
        338 => Some(EV_KEY::KEY_INSERT),
        339 => Some(EV_KEY::KEY_DELETE),
        327 => Some(EV_KEY::KEY_HOME),
        329 => Some(EV_KEY::KEY_PAGEUP),
        337 => Some(EV_KEY::KEY_PAGEDOWN),
        335 => Some(EV_KEY::KEY_END),
        _ => {
            let scancode = if scancode > KEYCODE_LIMIT as u16 {
                338
            } else {
                scancode
            };
            evdev_rs::enums::int_to_ev_key(scancode as u32)
        }
    }
}
