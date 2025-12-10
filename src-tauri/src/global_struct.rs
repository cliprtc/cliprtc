use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use base64::{prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::utils::{constant::KEY, encrypt};

#[derive(Debug, Clone)]
pub struct MdnsProperties {
    pub version: Option<String>,
    pub port: Option<String>,
    pub uuid: Option<String>,
    pub fingerprint: Option<String>,
}

impl MdnsProperties {
    // Constructor to create MdnsProperties from a HashMap
    pub fn from_hashmap(map: HashMap<String, String>) -> Self {
        MdnsProperties {
            version: map.get("version").cloned(),
            port: map.get("port").cloned(),
            uuid: map.get("uuid").cloned(),
            fingerprint: map.get("fingerprint").cloned(),
        }
    }

    // Convert validated MdnsProperties into MetaInfo
    pub fn to_meta_info(&self) -> Option<MetaInfo> {
        let validated_properties = self.validate_and_process();

        validated_properties
    }

    // Decode the fingerprint (Base64 -> Vec<u8>)
    fn decode_fingerprint(fingerprint: String) -> Option<Vec<u8>> {
        BASE64_STANDARD.decode(fingerprint).ok()
    }

    // Decrypt the fingerprint (assuming some decryption logic)
    fn decrypt_fingerprint(encrypted_data: &[u8]) -> Option<Vec<u8>> {
        // Example decryption logic (replace with actual decryption code)
        encrypt::decrypt(KEY.to_string(), encrypted_data)
    }

    // Validate and process version, port, and fingerprint
    pub fn validate_and_process(&self) -> Option<MetaInfo> {

        // Validate and parse port (convert to u16)
        let port = self.port.as_ref().and_then(|p| u16::from_str(&p).ok());

        // Validate and process fingerprint (decode and decrypt)
        let fingerprint = self.fingerprint.as_ref().and_then(|f| {
            MdnsProperties::decode_fingerprint(f.clone())
                .and_then(|data| MdnsProperties::decrypt_fingerprint(&data))
                .and_then(|data| Some(String::from_utf8_lossy(&data).to_string()))
        });

        // If all fields are valid, return them; otherwise, return None
        match (self.version.clone(), port, self.uuid.clone(), fingerprint) {
            (Some(valid_version), Some(valid_port), Some(valid_uuid), Some(valid_fingerprint)) => {
                Some(MetaInfo {
                    version: valid_version,
                    port: valid_port,
                    uuid: valid_uuid,
                    fingerprint: valid_fingerprint,
                })
            }
            _ => None, // If any field is invalid or missing, return None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub host: String,
    pub fullname: String,
    pub addresses: AddressInfo,
    pub meta_info: MetaInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub v4: Vec<Ipv4Addr>,
    pub v6: Vec<Ipv6Addr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInfo {
    pub version: String,
    pub port: u16,
    pub uuid: String,
    pub fingerprint: String,
}
