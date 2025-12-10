use clipboard_rs::ClipboardContext;
use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
};
use tauri::{Manager, Wry};
use tauri_plugin_store::StoreExt;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::{clipboard::struct_type::ClipboardInfo, global_struct::DeviceInfo, APP};

pub static STORE_INFO: Lazy<Arc<tauri_plugin_store::Store<Wry>>> = Lazy::new(|| {
    // In dev mode, use the dev.json suffix
    #[cfg(any(dev, debug_assertions))]
    let path = "stores/.info.dev.json";
    // Use the official path in non-dev mode
    #[cfg(not(any(dev, debug_assertions)))]
    let path = "stores/.info.json";

    let app = APP.get().unwrap();
    let store = app.store(path).unwrap();

    store
});

pub static UUID: Lazy<String> = Lazy::new(|| {
    let key = "uuid";
    match STORE_INFO.get(key) {
        Some(value) => serde_json::from_value::<String>(value).unwrap(),
        None => {
            let value = Uuid::new_v4().to_string();
            STORE_INFO.set(key, value.clone());
            value
        }
    }
});

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
        Some(value) => serde_json::from_value::<String>(value).unwrap(),
        None => {
            let value = "cliprtc";
            STORE_SETTINGS.set(key, value);
            value.to_string()
        }
    }
});

pub static MDNS_SERVICE_TYPE: &str = "_cliprtc._quic._udp.local.";

#[cfg(any(dev, debug_assertions))]
pub static MAIN_WINDOW_LABEL: &str = "main";
#[cfg(any(dev, debug_assertions))]
pub static SETTINGS_WINDOW_LABEL: &str = "settings";

pub static ALLOW_FINGERPRINT: Lazy<DashSet<String>> = Lazy::new(DashSet::new);
pub static DEVICES: Lazy<DashMap<String, DeviceInfo>> = Lazy::new(DashMap::new);
pub static ALLOW_DEVICE_IDS: Lazy<DashSet<String>> = Lazy::new(DashSet::new);

pub static DEVICE_ID: Lazy<String> = Lazy::new(|| {
    let hostname = hostname::get()
        .unwrap_or(UUID.to_string().into())
        .to_string_lossy()
        .to_string();
    hostname
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
    let ctx = ClipboardContext::new().unwrap();

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
