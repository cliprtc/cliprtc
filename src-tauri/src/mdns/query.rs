use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    global_struct::{AddressInfo, DeviceInfo, MdnsProperties},
    utils::{
        constant::{ALLOW_FINGERPRINT, CLIPBOARD_INFO, DEVICES, MDNS_SERVICE_TYPE, UUID},
        event::event_names,
    },
    APP,
};
use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};
use tauri::{async_runtime::spawn, Emitter};
use tauri_plugin_log::log;

pub fn query(mdns: &ServiceDaemon) {
    let service_type = MDNS_SERVICE_TYPE;
    let receiver = Arc::new(mdns.browse(&service_type).expect("Failed to browse")).clone();
    let app = APP.get().unwrap();

    spawn(async move {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let map = info.txt_properties.into_property_map_str();
                    let mdns_properties = MdnsProperties::from_hashmap(map);

                    let meta_info = match mdns_properties.to_meta_info() {
                        Some(meta_info) => meta_info,
                        None => {
                            log::warn!(
                                "Server Invalid: {}\nproperties: {:#?}",
                                info.fullname,
                                mdns_properties
                            );
                            continue;
                        }
                    };

                    if meta_info.uuid == UUID.to_string() {
                        continue;
                    }

                    let key = info.fullname.clone();
                    let fingerprint = meta_info.fingerprint.clone();

                    if let Some(mut device) = DEVICES.get_mut(&key) {
                        let uuid = meta_info.uuid.clone();

                        device.addresses = extract_addresses(&info.addresses);
                        device.meta_info = meta_info;
                        ALLOW_FINGERPRINT.insert(fingerprint);
                        log::info!("Update existing server info: {}", info.fullname);

                        // reset remote_seq_map
                        let map = &CLIPBOARD_INFO.remote_seq_map;
                        map.insert(uuid, 0);

                        let _ = app.emit(event_names::DEVICE_FOUND, ());
                        continue;
                    }

                    log::info!(
                        "Server Found: {}:{} [v{}]",
                        info.fullname,
                        meta_info.port,
                        meta_info.version
                    );

                    ALLOW_FINGERPRINT.insert(fingerprint);

                    let addresses = extract_addresses(&info.addresses);
                    let device = DeviceInfo {
                        host: info.host,
                        fullname: info.fullname,
                        addresses,
                        meta_info,
                    };
                    DEVICES.insert(key, device);
                    let _ = app.emit(event_names::DEVICE_FOUND, ());
                }
                ServiceEvent::ServiceRemoved(_name, fullname) => {
                    if DEVICES.contains_key(&fullname) {
                        log::warn!("Server Removed: {}", fullname);
                        let (_, device) = DEVICES.remove(&fullname).unwrap();
                        ALLOW_FINGERPRINT.remove(&device.meta_info.fingerprint);
                    }
                }
                _ => {}
            }
        }
    });
}

fn extract_addresses(addresses: &HashSet<ScopedIp>) -> AddressInfo {
    addresses.iter().map(|x| x.to_ip_addr()).fold(
        AddressInfo {
            v4: vec![],
            v6: vec![],
        },
        |mut acc, ip| {
            match ip {
                IpAddr::V4(v4) => acc.v4.push(v4),
                IpAddr::V6(v6) => acc.v6.push(v6),
            }
            acc
        },
    )
}
