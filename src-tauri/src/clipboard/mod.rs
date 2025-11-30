use std::{fs, sync::atomic::Ordering, thread};

use clipboard_rs::{
    Clipboard, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext, ContentFormat,
};
use handler::parse_clipboard_info;
use once_cell::sync::Lazy;
use struct_type::ClipboardManager;
use tauri_plugin_log::log;

use crate::utils::constant::{APP_TMP_DIR_FILES_CACHE, CLIPBOARD_INFO};

pub mod handler;
pub mod struct_type;

static CLIPBOARD_FORMATS: Lazy<Vec<ContentFormat>> = Lazy::new(|| {
    vec![
        ContentFormat::Text,
        // ContentFormat::Rtf,
        // ContentFormat::Html,
        ContentFormat::Image,
        ContentFormat::Files,
    ]
});

impl ClipboardManager {
    pub fn new() -> Self {
        ClipboardManager {}
    }
}

impl ClipboardHandler for ClipboardManager {
    fn on_clipboard_change(&mut self) {
        // Is it a change triggered by an internal program write
        if CLIPBOARD_INFO.lock.load(Ordering::SeqCst) {
            CLIPBOARD_INFO.lock.store(false, Ordering::SeqCst);
            return;
        }

        let content = CLIPBOARD_INFO.ctx.get(&CLIPBOARD_FORMATS);
        if let Ok(data) = content {
            fs::remove_dir_all(APP_TMP_DIR_FILES_CACHE.clone()).ok();

            // Increment the local serial number
            CLIPBOARD_INFO.seq.fetch_add(1, Ordering::SeqCst);

            match parse_clipboard_info(data) {
                Some(clipboard_info) => {
                    if let Err(e) = CLIPBOARD_INFO.tx.send(clipboard_info) {
                        log::error!("Failed to send clipboard change notification: {}", e);
                    }
                }
                None => {}
            }
        }
    }
}

pub fn start() {
    thread::spawn(move || {
        let manager = ClipboardManager::new();

        let mut watcher = ClipboardWatcherContext::new().unwrap();

        let _watcher_shutdown = watcher.add_handler(manager).get_shutdown_channel();

        watcher.start_watch();
    });
}
