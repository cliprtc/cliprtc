use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, Result};

pub const PROTOCOL_VERSION: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Text,
    Image,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardEvent {
    pub protocol_version: u16,
    pub event_id: Uuid,
    pub origin: Uuid,
    pub boot_id: Uuid,
    pub seq: u64,
    pub lamport: u64,
    pub kind: EventKind,
    pub content_hash: [u8; 32],
    pub size: u64,
}

impl ClipboardEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        origin: Uuid,
        boot_id: Uuid,
        seq: u64,
        lamport: u64,
        kind: EventKind,
        content_hash: [u8; 32],
        size: u64,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            event_id: Uuid::new_v4(),
            origin,
            boot_id,
            seq,
            lamport,
            kind,
            content_hash,
            size,
        }
    }
}

pub fn encode_event(event: &ClipboardEvent, max_len: usize) -> Result<Vec<u8>> {
    let body = serde_json::to_vec(event)
        .map_err(|e| CoreError::Protocol(format!("encode failed: {e}")))?;
    if body.len() > max_len {
        return Err(CoreError::Protocol(format!(
            "meta too large: {} > {max_len}",
            body.len()
        )));
    }
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn decode_event(buf: &[u8], max_len: usize) -> Result<ClipboardEvent> {
    if buf.len() < 4 {
        return Err(CoreError::Protocol("meta header truncated".into()));
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if len > max_len {
        return Err(CoreError::Protocol(format!(
            "meta too large: {len} > {max_len}"
        )));
    }
    if buf.len() != 4 + len {
        return Err(CoreError::Protocol(format!(
            "meta length mismatch: got {}, expected {}",
            buf.len(),
            4 + len
        )));
    }
    serde_json::from_slice(&buf[4..])
        .map_err(|e| CoreError::Protocol(format!("decode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> ClipboardEvent {
        ClipboardEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            1,
            EventKind::Text,
            [7; 32],
            5,
        )
    }

    #[test]
    fn roundtrip() {
        let event = event();
        let encoded = encode_event(&event, 1024).unwrap();
        let decoded = decode_event(&encoded, 1024).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn rejects_oversized_meta() {
        let event = event();
        assert!(encode_event(&event, 4).is_err());
    }

    #[test]
    fn rejects_truncated_meta() {
        assert!(decode_event(&[0, 0, 0], 1024).is_err());
    }
}
