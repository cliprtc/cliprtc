use cliprtc_core::identity::{extract_device_uuid, extract_public_key_hash, TrustDecision};
use cliprtc_core::{Core, CoreConfig};
use uuid::Uuid;

#[test]
fn two_devices_in_same_room_can_bind_each_other() {
    let room = "correct horse battery staple";
    let mut a = Core::new(CoreConfig::new(room).unwrap(), Uuid::new_v4()).unwrap();
    let mut b = Core::new(CoreConfig::new(room).unwrap(), Uuid::new_v4()).unwrap();

    let a_cert = a.device_cert().unwrap();
    let b_cert = b.device_cert().unwrap();

    let a_uuid = extract_device_uuid(a_cert.cert_der.as_ref()).unwrap();
    let b_uuid = extract_device_uuid(b_cert.cert_der.as_ref()).unwrap();
    let a_hash = extract_public_key_hash(a_cert.cert_der.as_ref()).unwrap();
    let b_hash = extract_public_key_hash(b_cert.cert_der.as_ref()).unwrap();

    assert_eq!(
        a.trust_mut().check_and_record(b_uuid, b_hash),
        TrustDecision::New
    );
    assert_eq!(
        b.trust_mut().check_and_record(a_uuid, a_hash),
        TrustDecision::New
    );
}

#[test]
fn different_room_cannot_derive_same_ca() {
    let a = Core::new(CoreConfig::new("room-a").unwrap(), Uuid::new_v4()).unwrap();
    let b = Core::new(CoreConfig::new("room-b").unwrap(), Uuid::new_v4()).unwrap();
    assert_ne!(a.room_ca.fingerprint(), b.room_ca.fingerprint());
}
