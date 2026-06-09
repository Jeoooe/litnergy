pub struct EdgeCoordinate(pub i16, pub i16);
pub struct MousePosition(pub i16, pub i16);

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
