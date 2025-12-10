use std::{
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use clipboard_rs::{common::RustImage, ClipboardContent};
use tauri_plugin_log::log;

use crate::utils::{
    compress::compress_paths,
    constant::{APP_TMP_DIR_FILES_CACHE, CLIPBOARD_INFO, UUID},
};

use super::struct_type::{ClipboardInfo, EClipboardKind};

pub fn parse_clipboard_info(clipboards: Vec<ClipboardContent>) -> Option<ClipboardInfo> {
    let source_id = UUID.clone();
    let seq = CLIPBOARD_INFO.seq.load(Ordering::SeqCst);
    for clipboard in clipboards {
        match clipboard {
            ClipboardContent::Text(text) => {
                let bytes = text.as_bytes().to_vec();
                return Some(ClipboardInfo {
                    kind: EClipboardKind::Text,
                    source_id,
                    seq,
                    content: bytes,
                });
            }

            ClipboardContent::Image(img) => {
                if let Ok(img_data) = img.to_png() {
                    let bytes = img_data.get_bytes().to_vec();
                    return Some(ClipboardInfo {
                        kind: EClipboardKind::Image,
                        source_id,
                        seq,
                        content: bytes,
                    });
                }
            }

            ClipboardContent::Files(paths) => {
                if paths.is_empty() {
                    continue;
                }
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let short_id = format!("{:x}.tar", millis);
                let pack_path = APP_TMP_DIR_FILES_CACHE.join(short_id);

                if let Err(e) = compress_paths(&paths, &pack_path) {
                    log::error!("Failed to compress files: {}", e);
                    continue;
                }
                return Some(ClipboardInfo {
                    kind: EClipboardKind::File,
                    source_id,
                    seq,
                    content: pack_path.to_string_lossy().as_bytes().to_vec(),
                });
            }
            _ => {}
        }
    }
    None
}
