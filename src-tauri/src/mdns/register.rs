use crate::utils::{
    constant::{DEVICE_ID, KEY, MDNS_SERVICE_TYPE, UUID},
    encrypt,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tauri_plugin_log::log;

pub fn register(port: u16, fingerprint: String) -> ServiceDaemon {
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");

    let service_type = MDNS_SERVICE_TYPE;
    let service_instance_name = DEVICE_ID.clone();
    let service_hostname = format!("{}.local.", service_instance_name);
    let service_ip = "";

    let encrypt_data = encrypt::encrypt(KEY.to_string(), fingerprint.as_str().as_bytes()).unwrap();
    let fingerprint = BASE64_STANDARD.encode(encrypt_data);
    let properties = [
        ("version", "1.0"),
        ("port", &port.to_string()),
        ("fingerprint", &fingerprint),
        ("uuid", &UUID.to_string())
    ];

    let service_info = ServiceInfo::new(
        service_type,
        &service_instance_name,
        &service_hostname,
        service_ip,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    mdns.register(service_info)
        .expect("Failed to register mDNS service");

    log::info!(
        "Registered service {}.{}",
        &service_instance_name,
        &service_type
    );

    mdns
}
