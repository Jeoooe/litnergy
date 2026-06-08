
#[derive(Debug)]
pub enum ProtocolCode {
    LSYN,
    QINF,
    CALV,
    Unspecified,
}

pub fn message_to_code(msg: &[u8]) -> ProtocolCode {
    match &msg[0..4] {
        b"QINF" => ProtocolCode::QINF,
        b"LSYN" => ProtocolCode::LSYN,
        b"CALV" => ProtocolCode::CALV,
        _ => ProtocolCode::Unspecified,
    }
}
