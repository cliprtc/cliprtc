#[cfg(unix)]
use clipboard_rs::ClipboardContextX11Options;
use clipboard_rs::ClipboardContext;
use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, Wry};
use tauri_plugin_store::StoreExt;
use tokio::sync::{mpsc, RwLock};

use crate::{clipboard::struct_type::ClipboardInfo, global_struct::DeviceInfo, APP};

pub static STORE_SETTINGS: Lazy<Arc<tauri_plugin_store::Store<Wry>>> = Lazy::new(|| {
    // In dev mode, use the dev.json suffix
    #[cfg(any(dev, debug_assertions))]
    let path = "stores/.settings.dev.json";
    // Use the official path in non-dev mode
    #[cfg(not(any(dev, debug_assertions)))]
    let path = "stores/.settings.json";

    let app = APP.get().unwrap();
    let store = app.store(path).unwrap();

    store
});

pub static KEY: Lazy<String> = Lazy::new(|| {
    let key = "key";
    match STORE_SETTINGS.get(key) {
        Some(value) => value.to_string(),
        None => {
            let value = "cliprtc";
            STORE_SETTINGS.set(key, value);
            value.to_string()
        }
    }
});

pub static MDNS_SERVICE_TYPE: &str = "_cliprtc._tcp.local.";

pub static SETTINGS_WINDOW_LABEL: &str = "settings";

pub static DEVICES: Lazy<DashMap<String, DeviceInfo>> = Lazy::new(DashMap::new);
pub static ALLOW_FINGERPRINT: Lazy<DashSet<String>> = Lazy::new(DashSet::new);

pub static DEVICE_ID: Lazy<String> = Lazy::new(|| {
    let hostname = hostname::get()
        .unwrap_or("unknown".into())
        .to_string_lossy()
        .to_string();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let short_id = format!("{:x}", millis);
    let service_instance_name = format!("cliprtc-{}-{}", hostname, short_id);
    service_instance_name
});

pub static APP_TMP_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let app = APP.get().unwrap();
    let mut app_tmp_dir = app.path().temp_dir().unwrap();
    let identifier = app.config().identifier.clone();
    app_tmp_dir.push(identifier);
    app_tmp_dir
});

pub static APP_TMP_DIR_FILES_CACHE: Lazy<PathBuf> = Lazy::new(|| {
    let mut dir = APP_TMP_DIR.clone();
    dir.push("files_cache");
    dir
});

pub struct GlobalClipboardInfo {
    pub ctx: ClipboardContext,
    pub tx: mpsc::UnboundedSender<ClipboardInfo>,
    pub rx: RwLock<mpsc::UnboundedReceiver<ClipboardInfo>>,
    pub lock: AtomicBool,
    pub seq: AtomicU64,
    pub remote_seq_map: DashMap<String, u64>,
}

pub static CLIPBOARD_INFO: Lazy<GlobalClipboardInfo> = Lazy::new(|| {
    #[cfg(not(unix))]
    let ctx = ClipboardContext::new().unwrap();
    #[cfg(unix)]
    let ctx = ClipboardContext::new_with_options(ClipboardContextX11Options { read_timeout: None })
        .unwrap();

    let (tx, rx) = mpsc::unbounded_channel();
    GlobalClipboardInfo {
        ctx,
        tx,
        rx: RwLock::new(rx),
        lock: AtomicBool::new(false),
        seq: AtomicU64::new(0),
        remote_seq_map: DashMap::new(),
    }
});
