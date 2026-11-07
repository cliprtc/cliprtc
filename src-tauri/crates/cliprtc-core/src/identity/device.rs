use rcgen::{KeyPair, PKCS_ED25519};
use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use x509_parser::prelude::*;

use crate::error::{CoreError, Result};

pub struct DeviceIdentity {
    pub uuid: Uuid,
    pub(crate) key: KeyPair,
}

impl DeviceIdentity {
    pub fn generate(uuid: Uuid) -> Result<Self> {
        let key = KeyPair::generate_for(&PKCS_ED25519)
            .map_err(|e| CoreError::Identity(format!("device key failed: {e}")))?;
        Ok(Self { uuid, key })
    }

    pub fn from_pkcs8(uuid: Uuid, der: &[u8]) -> Result<Self> {
        let key = KeyPair::from_der_and_sign_algo(
            &PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der.to_vec())),
            &PKCS_ED25519,
        )
        .map_err(|e| CoreError::Identity(format!("device key load failed: {e}")))?;
        Ok(Self { uuid, key })
    }

    pub fn private_key_der(&self) -> Vec<u8> {
        self.key.serialize_der()
    }

    pub fn public_key_hash(&self) -> [u8; 32] {
        Sha256::digest(self.key.public_key_raw()).into()
    }
}

pub fn extract_device_uuid(cert_der: &[u8]) -> Result<Uuid> {
    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| CoreError::Identity(format!("parse cert failed: {e}")))?;

    if let Ok(Some(san)) = cert.subject_alternative_name() {
        for name in &san.value.general_names {
            if let GeneralName::URI(uri) = name {
                if let Some(id) = uri.strip_prefix("cliprtc://device/") {
                    return Uuid::parse_str(id)
                        .map_err(|e| CoreError::Identity(format!("bad uuid: {e}")));
                }
            }
        }
    }

    for attr in cert.subject().iter_common_name() {
        if let Ok(id) = attr.as_str() {
            if let Ok(uuid) = Uuid::parse_str(id) {
                return Ok(uuid);
            }
        }
    }

    Err(CoreError::Identity("certificate has no device uuid".into()))
}

pub fn extract_public_key_hash(cert_der: &[u8]) -> Result<[u8; 32]> {
    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| CoreError::Identity(format!("parse cert failed: {e}")))?;
    let raw = cert.public_key().subject_public_key.data.as_ref();
    Ok(Sha256::digest(raw).into())
}
