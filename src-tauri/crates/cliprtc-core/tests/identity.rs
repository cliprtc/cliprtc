use cliprtc_core::identity::{
    extract_device_uuid, extract_public_key_hash, DeviceIdentity, RoomCa, TrustDecision, TrustStore,
};
use uuid::Uuid;

#[test]
fn device_cert_roundtrips_identity() {
    let uuid = Uuid::new_v4();
    let ca = RoomCa::derive("test-room").unwrap();
    let device = DeviceIdentity::generate(uuid).unwrap();
    let cert = ca.issue_device_cert(&device).unwrap();

    assert_eq!(extract_device_uuid(cert.cert_der.as_ref()).unwrap(), uuid);
    assert_eq!(
        extract_public_key_hash(cert.cert_der.as_ref()).unwrap(),
        device.public_key_hash()
    );
}

#[test]
fn wrong_room_key_makes_a_different_ca() {
    let a = RoomCa::derive("room-a").unwrap();
    let b = RoomCa::derive("room-b").unwrap();
    assert_ne!(a.fingerprint(), b.fingerprint());
}

#[test]
fn trust_store_auto_accepts_new_device_and_rejects_key_change() {
    let mut store = TrustStore::new();
    let uuid = Uuid::new_v4();
    let ca = RoomCa::derive("test-room").unwrap();
    let device = DeviceIdentity::generate(uuid).unwrap();
    let cert = ca.issue_device_cert(&device).unwrap();
    let hash = extract_public_key_hash(cert.cert_der.as_ref()).unwrap();

    assert_eq!(store.check_and_record(uuid, hash), TrustDecision::New);
    assert_eq!(store.check_and_record(uuid, hash), TrustDecision::Known);
    assert_eq!(
        store.check_and_record(uuid, [9; 32]),
        TrustDecision::Changed
    );
}
