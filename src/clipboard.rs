use std::io::Write;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::client::DeskflowClient;

const CLIPBOARD_ID: u8 = 0;
const FORMAT_TEXT: u32 = 0;
const FORMAT_BITMAP: u32 = 2;
const CHUNK_SIZE: usize = 32 * 1024;
const CHUNK_SINGLE: u8 = 0;
const CHUNK_START: u8 = 1;
const CHUNK_DATA: u8 = 2;
const CHUNK_END: u8 = 3;
const BMP_FILE_HEADER_SIZE: usize = 14;
const MAX_CLIPBOARD_DATA_SIZE: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClipboardPayload {
    Text(String),
    Bitmap(Vec<u8>),
}

#[derive(Debug, Default)]
pub struct ClipboardState {
    expected_size: Option<usize>,
    data: Vec<u8>,
    last_remote_payload: Option<ClipboardPayload>,
    last_sent_payload: Option<ClipboardPayload>,
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
            CHUNK_SINGLE => {
                if data.len() <= MAX_CLIPBOARD_DATA_SIZE {
                    self.set_local_payload(data)?;
                } else {
                    warn!("Ignoring oversized clipboard payload from server.");
                }
            }
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
                self.data.clear();
                if size > MAX_CLIPBOARD_DATA_SIZE {
                    self.expected_size = None;
                    warn!("Ignoring oversized clipboard payload from server.");
                } else {
                    self.expected_size = Some(size);
                }
            }
            CHUNK_DATA => {
                if self.expected_size.is_some()
                    && self.data.len() + data.len() <= MAX_CLIPBOARD_DATA_SIZE
                {
                    self.data.extend_from_slice(data);
                }
            }
            CHUNK_END => {
                let Some(expected_size) = self.expected_size.take() else {
                    self.data.clear();
                    return Ok(());
                };
                if expected_size != self.data.len() {
                    self.data.clear();
                    warn!("Ignoring clipboard payload with mismatched chunk size.");
                    return Ok(());
                }

                let data = std::mem::take(&mut self.data);
                self.set_local_payload(&data)?;
            }
            _ => warn!("Unknown clipboard chunk mark: {}", mark),
        }

        Ok(())
    }

    fn set_local_payload(&mut self, data: &[u8]) -> std::io::Result<()> {
        let Some(payload) = extract_payload(data)? else {
            return Ok(());
        };
        match write_local_payload(&payload) {
            Ok(()) => {
                match &payload {
                    ClipboardPayload::Text(_) => info!("Received clipboard text from server."),
                    ClipboardPayload::Bitmap(_) => info!("Received clipboard bitmap from server."),
                }
                self.last_remote_payload = Some(payload);
                self.last_sent_payload = None;
            }
            Err(e) => warn!("Failed to set local clipboard: {}", e),
        }
        Ok(())
    }

    fn should_send_local_payload(&self, payload: &ClipboardPayload) -> bool {
        self.last_remote_payload.as_ref() != Some(payload)
            && self.last_sent_payload.as_ref() != Some(payload)
    }

    fn remember_sent_payload(&mut self, payload: ClipboardPayload) {
        self.last_remote_payload = None;
        self.last_sent_payload = Some(payload);
    }
}

pub fn send_local_payload(client: &mut DeskflowClient) -> std::io::Result<()> {
    let payload = match read_local_payload() {
        Ok(Some(payload)) => payload,
        Ok(None) => return Ok(()),
        Err(e) => {
            warn!("Failed to read local clipboard: {}", e);
            return Ok(());
        }
    };
    if !client.clipboard.should_send_local_payload(&payload) {
        return Ok(());
    }
    if is_empty_payload(&payload) {
        return Ok(());
    }
    if !is_payload_safe_to_send(&payload) {
        return Ok(());
    }

    send_grab(client)?;
    send_payload(client, &payload)?;
    match &payload {
        ClipboardPayload::Text(_) => info!("Sent clipboard text to server."),
        ClipboardPayload::Bitmap(_) => info!("Sent clipboard bitmap to server."),
    }
    client.clipboard.remember_sent_payload(payload);
    Ok(())
}

fn is_empty_payload(payload: &ClipboardPayload) -> bool {
    match payload {
        ClipboardPayload::Text(text) => text.is_empty(),
        ClipboardPayload::Bitmap(bitmap) => bitmap.is_empty(),
    }
}

fn is_payload_safe_to_send(payload: &ClipboardPayload) -> bool {
    let size = payload_data_len(payload);
    if size > MAX_CLIPBOARD_DATA_SIZE {
        warn!(
            "Skipping clipboard payload larger than {} bytes.",
            MAX_CLIPBOARD_DATA_SIZE
        );
        return false;
    }

    if let ClipboardPayload::Bitmap(bitmap) = payload {
        if !is_valid_dib(bitmap) {
            warn!("Skipping malformed bitmap clipboard payload.");
            return false;
        }
    }

    true
}

fn payload_data_len(payload: &ClipboardPayload) -> usize {
    let payload_size = match payload {
        ClipboardPayload::Text(text) => text.len(),
        ClipboardPayload::Bitmap(bitmap) => bitmap.len(),
    };
    4 + 4 + 4 + payload_size
}

fn send_grab(client: &mut DeskflowClient) -> std::io::Result<()> {
    let mut msg = b"CCLP".to_vec();
    msg.push(CLIPBOARD_ID);
    msg.extend_from_slice(&client.enter_sequence.to_be_bytes());
    client.write_vec(&mut msg)
}

fn send_payload(client: &mut DeskflowClient, payload: &ClipboardPayload) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.extend_from_slice(&1u32.to_be_bytes());
    match payload {
        ClipboardPayload::Text(text) => {
            data.extend_from_slice(&FORMAT_TEXT.to_be_bytes());
            data.extend_from_slice(&(text.len() as u32).to_be_bytes());
            data.extend_from_slice(text.as_bytes());
        }
        ClipboardPayload::Bitmap(bitmap) => {
            data.extend_from_slice(&FORMAT_BITMAP.to_be_bytes());
            data.extend_from_slice(&(bitmap.len() as u32).to_be_bytes());
            data.extend_from_slice(bitmap);
        }
    }

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

fn extract_payload(data: &[u8]) -> std::io::Result<Option<ClipboardPayload>> {
    let mut offset = 0;
    let Some(format_count) = read_u32(data, &mut offset) else {
        return Ok(None);
    };
    let mut text = None;

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
            text = Some(
                String::from_utf8(data[offset..end].to_vec())
                    .map(normalize_text)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            );
        } else if format == FORMAT_BITMAP {
            let bitmap = data[offset..end].to_vec();
            if is_valid_dib(&bitmap) {
                return Ok(Some(ClipboardPayload::Bitmap(bitmap)));
            }
            warn!("Ignoring malformed bitmap clipboard payload from server.");
        }
        offset = end;
    }

    Ok(text.map(ClipboardPayload::Text))
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

fn read_local_payload() -> std::io::Result<Option<ClipboardPayload>> {
    if let Some(bitmap) = read_local_bitmap()? {
        return Ok(Some(ClipboardPayload::Bitmap(bitmap)));
    }

    read_local_text().map(|text| text.map(ClipboardPayload::Text))
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

fn read_local_bitmap() -> std::io::Result<Option<Vec<u8>>> {
    for command in [
        &["wl-paste", "--type", "image/bmp"][..],
        &["wl-paste", "--type", "image/x-MS-bmp"][..],
        &["wl-paste", "--type", "image/x-bmp"][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-out",
            "-target",
            "image/bmp",
        ][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-out",
            "-target",
            "image/x-MS-bmp",
        ][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-out",
            "-target",
            "image/x-bmp",
        ][..],
    ] {
        if let Some(dib) = read_bitmap_command(command)? {
            return Ok(Some(dib));
        }
    }

    for command in [
        &["wl-paste", "--type", "image/png"][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-out",
            "-target",
            "image/png",
        ][..],
    ] {
        if let Some(png) = read_bytes_command(command)? {
            if let Ok(Some(bmp)) = convert_image_to_bmp(&png, "png") {
                if let Some(dib) = bmp_to_dib(&bmp) {
                    if is_valid_dib(&dib) {
                        return Ok(Some(dib));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn read_bitmap_command(command: &[&str]) -> std::io::Result<Option<Vec<u8>>> {
    let Some(bmp) = read_bytes_command(command)? else {
        return Ok(None);
    };
    Ok(bmp_to_dib(&bmp).filter(|dib| is_valid_dib(dib)))
}

fn read_bytes_command(command: &[&str]) -> std::io::Result<Option<Vec<u8>>> {
    let Ok(output) = Command::new(command[0]).args(&command[1..]).output() else {
        return Ok(None);
    };
    if output.status.success() && !output.stdout.is_empty() {
        Ok(Some(output.stdout))
    } else {
        Ok(None)
    }
}

fn write_local_payload(payload: &ClipboardPayload) -> std::io::Result<()> {
    match payload {
        ClipboardPayload::Text(text) => write_local_text(text),
        ClipboardPayload::Bitmap(bitmap) => write_local_bitmap(bitmap),
    }
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

fn write_local_bitmap(bitmap: &[u8]) -> std::io::Result<()> {
    let bmp = dib_to_bmp(bitmap);
    for command in [
        &["wl-copy", "--type", "image/bmp"][..],
        &["wl-copy", "--type", "image/x-MS-bmp"][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-in",
            "-target",
            "image/bmp",
        ][..],
        &[
            "xclip",
            "-selection",
            "clipboard",
            "-in",
            "-target",
            "image/x-MS-bmp",
        ][..],
    ] {
        if write_bytes_to_command(command, &bmp).is_ok() {
            return Ok(());
        }
    }

    if let Ok(Some(png)) = convert_bmp_to_image(&bmp, "png") {
        for command in [
            &["wl-copy", "--type", "image/png"][..],
            &[
                "xclip",
                "-selection",
                "clipboard",
                "-in",
                "-target",
                "image/png",
            ][..],
        ] {
            if write_bytes_to_command(command, &png).is_ok() {
                return Ok(());
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no supported image clipboard command found",
    ))
}

fn write_to_command(command: &[&str], text: &str) -> std::io::Result<()> {
    write_bytes_to_command(command, text.as_bytes())
}

fn write_bytes_to_command(command: &[&str], data: &[u8]) -> std::io::Result<()> {
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
    stdin.write_all(data)?;
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

fn convert_image_to_bmp(data: &[u8], format: &str) -> std::io::Result<Option<Vec<u8>>> {
    convert_image(data, &format!("{}:-", format), "bmp:-")
}

fn convert_bmp_to_image(data: &[u8], format: &str) -> std::io::Result<Option<Vec<u8>>> {
    convert_image(data, "bmp:-", &format!("{}:-", format))
}

fn convert_image(data: &[u8], input: &str, output: &str) -> std::io::Result<Option<Vec<u8>>> {
    for command in [
        &["magick", input, output][..],
        &["convert", input, output][..],
    ] {
        if let Ok(converted) = run_filter_command(command, data) {
            if !converted.is_empty() {
                return Ok(Some(converted));
            }
        }
    }
    Ok(None)
}

fn run_filter_command(command: &[&str], data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut child = Command::new(command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "converter stdin unavailable",
        ));
    };
    stdin.write_all(data)?;
    drop(stdin);

    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "image conversion failed",
        ))
    }
}

fn is_valid_dib(dib: &[u8]) -> bool {
    let Some(pixel_offset) = dib_pixel_offset(dib) else {
        return false;
    };
    pixel_offset > 0 && pixel_offset <= dib.len()
}

fn bmp_to_dib(bmp: &[u8]) -> Option<Vec<u8>> {
    if bmp.len() <= BMP_FILE_HEADER_SIZE || bmp.get(0..2) != Some(b"BM") {
        return None;
    }

    let file_pixel_offset = read_u32_le(bmp, 10)? as usize;
    let dib = &bmp[BMP_FILE_HEADER_SIZE..];
    let natural_offset = dib_pixel_offset(dib).unwrap_or(40);

    if natural_offset != 0
        && file_pixel_offset > BMP_FILE_HEADER_SIZE + natural_offset
        && file_pixel_offset <= bmp.len()
    {
        let mut compact = dib[..natural_offset].to_vec();
        compact.extend_from_slice(&bmp[file_pixel_offset..]);
        Some(compact)
    } else {
        Some(dib.to_vec())
    }
}

fn dib_to_bmp(dib: &[u8]) -> Vec<u8> {
    let pixel_offset = dib_pixel_offset(dib).unwrap_or(40);
    let mut bmp = Vec::with_capacity(BMP_FILE_HEADER_SIZE + dib.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((BMP_FILE_HEADER_SIZE + dib.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&((BMP_FILE_HEADER_SIZE + pixel_offset) as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    bmp
}

fn dib_pixel_offset(dib: &[u8]) -> Option<usize> {
    const BITMAPINFOHEADER_SIZE: u32 = 40;
    const BI_BITFIELDS: u32 = 3;
    const BI_ALPHABITFIELDS: u32 = 6;

    if dib.len() < 16 {
        return None;
    }

    let header_size = read_u32_le(dib, 0)?;
    if header_size < 12 || header_size as usize > dib.len() {
        return None;
    }

    let mut pixel_offset = header_size as usize;
    if header_size >= BITMAPINFOHEADER_SIZE {
        let bit_count = read_u16_le(dib, 14)?;
        let compression = read_u32_le(dib, 16)?;

        if header_size == BITMAPINFOHEADER_SIZE {
            if compression == BI_BITFIELDS {
                pixel_offset += 12;
            } else if compression == BI_ALPHABITFIELDS {
                pixel_offset += 16;
            }
        }

        if bit_count > 0 && bit_count <= 8 {
            let colors_used = read_u32_le(dib, 32).unwrap_or(0);
            let colors = if colors_used == 0 {
                1usize << bit_count
            } else {
                colors_used as usize
            };
            pixel_offset += colors * 4;
        }
    }

    Some(pixel_offset)
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}
