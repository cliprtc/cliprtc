use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    global_struct::{AddressInfo, DeviceInfo, MdnsProperties},
    utils::constant::{ALLOW_FINGERPRINT, DEVICES, MDNS_SERVICE_TYPE},
};
use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};
use tauri::async_runtime::spawn;
use tauri_plugin_log::log;

pub fn query(mdns: &ServiceDaemon, self_id: String) {
    let service_type = MDNS_SERVICE_TYPE;
    let receiver = Arc::new(mdns.browse(&service_type).expect("Failed to browse")).clone();

    spawn(async move {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    if info.fullname.contains(&self_id) {
                        continue;
                    }
                    if let Some(mut device) = DEVICES.get_mut(&info.fullname) {
                        device.addresses = extract_addresses(&info.addresses);
                        log::info!("Updated IP for existing device {}", info.fullname);
                        continue;
                    }

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

                    let addresses = extract_addresses(&info.addresses);
                    let fingerprint = meta_info.fingerprint.clone();
                    let device = DeviceInfo {
                        hostname: info.host,
                        port: info.port,
                        addresses,
                        meta_info,
                    };
                    log::info!(
                        "Server Found: {}:{} [v{}]",
                        info.fullname,
                        device.meta_info.port,
                        device.meta_info.version
                    );

                    ALLOW_FINGERPRINT.insert(fingerprint);
                    DEVICES.insert(info.fullname, device);
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
