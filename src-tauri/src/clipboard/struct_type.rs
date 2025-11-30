use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EClipboardKind {
    Text,
    Image,
    File,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardInfo {
    pub kind: EClipboardKind,
    pub source_id: String,
    pub seq: u64,
    pub content: Vec<u8>,
}

pub struct ClipboardManager;
