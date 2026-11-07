use ed25519_dalek::pkcs8::EncodePrivateKey;
use hkdf::Hkdf;
use rcgen::string::Ia5String;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, IsCa, KeyPair, KeyUsagePurpose,
    SanType, PKCS_ED25519,
};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};
use crate::identity::device::DeviceIdentity;

pub struct RoomCa {
    issuer: CertifiedIssuer<'static, KeyPair>,
}

pub struct DeviceCert {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivatePkcs8KeyDer<'static>,
}

impl RoomCa {
    pub fn derive(room_key: &str) -> Result<Self> {
        let mut seed = [0u8; 32];
        Hkdf::<Sha256>::new(Some(b"cliprtc-room-ca-v1"), room_key.as_bytes())
            .expand(b"ed25519-seed", &mut seed)
            .map_err(|_| CoreError::Identity("hkdf expand failed".into()))?;

        let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
        let pkcs8 = signing
            .to_pkcs8_der()
            .map_err(|e| CoreError::Identity(format!("pkcs8 encode failed: {e}")))?;
        let key_der = PrivatePkcs8KeyDer::from(pkcs8.as_bytes().to_vec());
        let key = KeyPair::from_der_and_sign_algo(&PrivateKeyDer::Pkcs8(key_der), &PKCS_ED25519)
            .map_err(|e| CoreError::Identity(format!("ca key load failed: {e}")))?;

        let mut params = CertificateParams::new(Vec::<String>::new())
            .map_err(|e| CoreError::Identity(format!("ca params failed: {e}")))?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        params
            .distinguished_name
            .push(DnType::CommonName, "ClipRTC Room CA");

        let issuer: CertifiedIssuer<'static, KeyPair> =
            CertifiedIssuer::self_signed(params, key)
                .map_err(|e| CoreError::Identity(format!("ca sign failed: {e}")))?;
        Ok(Self { issuer })
    }

    pub fn cert_der(&self) -> &CertificateDer<'static> {
        self.issuer.as_ref().der()
    }

    pub fn fingerprint(&self) -> [u8; 32] {
        Sha256::digest(self.cert_der().as_ref()).into()
    }

    pub fn issue_device_cert(&self, device: &DeviceIdentity) -> Result<DeviceCert> {
        let mut params = CertificateParams::new(vec!["cliprtc.local".to_string()])
            .map_err(|e| CoreError::Identity(format!("device params failed: {e}")))?;
        let uri = Ia5String::try_from(format!("cliprtc://device/{}", device.uuid))
            .map_err(|e| CoreError::Identity(format!("device uri failed: {e}")))?;
        params.subject_alt_names.push(SanType::URI(uri));
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params
            .distinguished_name
            .push(DnType::CommonName, device.uuid.to_string());

        let cert = params
            .signed_by(&device.key, &*self.issuer)
            .map_err(|e| CoreError::Identity(format!("device cert failed: {e}")))?;
        Ok(DeviceCert {
            cert_der: cert.der().clone(),
            key_der: PrivatePkcs8KeyDer::from(device.key.serialize_der()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_room_key_derives_same_ca() {
        let a = RoomCa::derive("same-room").unwrap();
        let b = RoomCa::derive("same-room").unwrap();
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn different_room_key_derives_different_ca() {
        let a = RoomCa::derive("room-a").unwrap();
        let b = RoomCa::derive("room-b").unwrap();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
