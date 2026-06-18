use std::io::Write;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::client::DeskflowClient;

const CLIPBOARD_ID: u8 = 0;
const FORMAT_TEXT: u32 = 0;
const CHUNK_SIZE: usize = 32 * 1024;
const CHUNK_SINGLE: u8 = 0;
const CHUNK_START: u8 = 1;
const CHUNK_DATA: u8 = 2;
const CHUNK_END: u8 = 3;

#[derive(Debug, Default)]
pub struct ClipboardState {
    expected_size: Option<usize>,
    data: Vec<u8>,
    last_remote_text: Option<String>,
    last_sent_text: Option<String>,
}

impl ClipboardState {
    pub fn handle_clipboard_grab(&mut self, msg: &[u8]) -> std::io::Result<()> {
        if msg.len() < 9 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "short clipboard grab message",
            ));
        }

        if msg[4] != CLIPBOARD_ID {
            return Ok(());
        }

        self.expected_size = None;
        self.data.clear();
        info!("Server grabbed clipboard.");
        Ok(())
    }

    pub fn handle_clipboard_message(&mut self, msg: &[u8]) -> std::io::Result<()> {
        let Some((id, _sequence, mark, data)) = decode_clipboard_chunk(msg)? else {
            return Ok(());
        };
        if id != CLIPBOARD_ID {
            return Ok(());
        }

        match mark {
            CHUNK_SINGLE => self.set_local_text(data)?,
            CHUNK_START => {
                let size = std::str::from_utf8(data)
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid clipboard chunk size",
                        )
                    })?;
                self.expected_size = Some(size);
                self.data.clear();
            }
            CHUNK_DATA => self.data.extend_from_slice(data),
            CHUNK_END => {
                let expected_size = self.expected_size.take();
                if expected_size != Some(self.data.len()) {
                    self.data.clear();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "clipboard chunk size mismatch",
                    ));
                }

                let data = std::mem::take(&mut self.data);
                self.set_local_text(&data)?;
            }
            _ => warn!("Unknown clipboard chunk mark: {}", mark),
        }

        Ok(())
    }

    fn set_local_text(&mut self, data: &[u8]) -> std::io::Result<()> {
        let Some(text) = extract_text(data)? else {
            return Ok(());
        };
        match write_local_text(&text) {
            Ok(()) => {
                self.last_remote_text = Some(text);
                self.last_sent_text = None;
                info!("Received clipboard text from server.");
            }
            Err(e) => warn!("Failed to set local clipboard text: {}", e),
        }
        Ok(())
    }

    fn should_send_local_text(&self, text: &str) -> bool {
        self.last_remote_text.as_deref() != Some(text)
            && self.last_sent_text.as_deref() != Some(text)
    }

    fn remember_sent_text(&mut self, text: String) {
        self.last_remote_text = None;
        self.last_sent_text = Some(text);
    }
}

pub fn send_local_text(client: &mut DeskflowClient) -> std::io::Result<()> {
    let text = match read_local_text() {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(()),
        Err(e) => {
            warn!("Failed to read local clipboard text: {}", e);
            return Ok(());
        }
    };
    if text.is_empty() {
        return Ok(());
    }
    if !client.clipboard.should_send_local_text(&text) {
        return Ok(());
    }

    send_grab(client)?;
    send_text(client, &text)?;
    client.clipboard.remember_sent_text(text);
    info!("Sent clipboard text to server.");
    Ok(())
}

fn send_grab(client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut msg = b"CCLP".to_vec();
    msg.push(CLIPBOARD_ID);
    msg.extend_from_slice(&client.enter_sequence.to_be_bytes());
    client.write_vec(&mut msg)
}

fn send_text(client: &mut DeskflowClient, text: &str) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.extend_from_slice(&1u32.to_be_bytes());
    data.extend_from_slice(&FORMAT_TEXT.to_be_bytes());
    data.extend_from_slice(&(text.len() as u32).to_be_bytes());
    data.extend_from_slice(text.as_bytes());

    append_clipboard_chunk(client, CHUNK_START, data.len().to_string().as_bytes())?;
    for chunk in data.chunks(CHUNK_SIZE) {
        append_clipboard_chunk(client, CHUNK_DATA, chunk)?;
    }
    append_clipboard_chunk(client, CHUNK_END, &[])
}

fn append_clipboard_chunk(
    client: &mut DeskflowClient,
    mark: u8,
    data: &[u8],
) -> std::io::Result<()> {
    let mut msg = b"DCLP".to_vec();
    msg.push(CLIPBOARD_ID);
    msg.extend_from_slice(&client.enter_sequence.to_be_bytes());
    msg.push(mark);
    msg.extend_from_slice(&(data.len() as u32).to_be_bytes());
    msg.extend_from_slice(data);
    client.write_vec(&mut msg)
}

fn decode_clipboard_chunk(msg: &[u8]) -> std::io::Result<Option<(u8, u32, u8, &[u8])>> {
    if msg.len() < 14 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "short clipboard message",
        ));
    }

    let id = msg[4];
    let sequence = u32::from_be_bytes([msg[5], msg[6], msg[7], msg[8]]);
    let mark = msg[9];
    let data_len = u32::from_be_bytes([msg[10], msg[11], msg[12], msg[13]]) as usize;
    let end = 14 + data_len;
    if msg.len() < end {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "truncated clipboard data",
        ));
    }

    Ok(Some((id, sequence, mark, &msg[14..end])))
}

fn extract_text(data: &[u8]) -> std::io::Result<Option<String>> {
    let mut offset = 0;
    let Some(format_count) = read_u32(data, &mut offset) else {
        return Ok(None);
    };

    for _ in 0..format_count {
        let Some(format) = read_u32(data, &mut offset) else {
            return Ok(None);
        };
        let Some(size) = read_u32(data, &mut offset) else {
            return Ok(None);
        };
        let size = size as usize;
        let end = offset + size;
        if data.len() < end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "truncated clipboard format data",
            ));
        }

        if format == FORMAT_TEXT {
            return String::from_utf8(data[offset..end].to_vec())
                .map(|text| Some(normalize_text(text)))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
        }
        offset = end;
    }

    Ok(None)
}

fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
    let end = *offset + 4;
    let bytes = data.get(*offset..end)?;
    *offset = end;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn normalize_text(text: String) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn read_local_text() -> std::io::Result<Option<String>> {
    for command in [
        &["wl-paste", "--no-newline", "--type", "text/plain"][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-out",
            "-target",
            "UTF8_STRING",
        ][..],
        &["xsel", "--clipboard", "--output"][..],
    ] {
        if let Ok(output) = Command::new(command[0]).args(&command[1..]).output() {
            if output.status.success() {
                return String::from_utf8(output.stdout)
                    .map(|text| Some(normalize_text(text)))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
        }
    }
    Ok(None)
}

fn write_local_text(text: &str) -> std::io::Result<()> {
    for command in [
        &["wl-copy", "--type", "text/plain;charset=utf-8"][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-in",
            "-target",
            "UTF8_STRING",
        ][..],
        &["xsel", "--clipboard", "--input"][..],
    ] {
        if write_to_command(command, text).is_ok() {
            return Ok(());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no supported clipboard command found",
    ))
}

fn write_to_command(command: &[&str], text: &str) -> std::io::Result<()> {
    let mut child = Command::new(command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .spawn()?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "clipboard command stdin unavailable",
        ));
    };
    stdin.write_all(text.as_bytes())?;
    drop(stdin);

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "clipboard command failed",
        ))
    }
}
