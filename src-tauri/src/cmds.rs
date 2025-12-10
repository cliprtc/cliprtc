use tauri::AppHandle;

use crate::{
    global_struct::DeviceInfo,
    init,
    utils::constant::{ALLOW_DEVICE_IDS, DEVICES},
};

#[tauri::command]
pub fn restart_app(app_handle: AppHandle) {
    app_handle.request_restart();
}

#[tauri::command]
pub fn exit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
pub fn start_server() {
    init::start();
}

#[tauri::command]
pub fn allow_device_add(id: String) -> bool {
    ALLOW_DEVICE_IDS.insert(id)
}

#[tauri::command]
pub fn allow_device_remove(id: String) -> bool {
    ALLOW_DEVICE_IDS.remove(&id).is_some()
}

#[tauri::command]
pub fn get_devices() -> Vec<DeviceInfo> {
    DEVICES.iter().map(|v| v.value().clone()).collect()
}
