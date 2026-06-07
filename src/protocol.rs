use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum ProtocolCode {
    LSYN,
    QINF,
    Unspecified,
}

pub fn message_to_code(msg: &[u8]) -> ProtocolCode {
    match &msg[0..4] {
        b"QINF" => ProtocolCode::QINF,
        b"LSYN" => ProtocolCode::LSYN,
        _ => ProtocolCode::Unspecified,
    }
}
