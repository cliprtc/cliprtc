use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use cliprtc_core::identity::{DeviceIdentity, RoomCa, TrustStore};
use cliprtc_core::protocol::{ClipboardEvent, EventKind};
use cliprtc_core::transport::Transport;
use uuid::Uuid;

fn build(room: &str, uuid: Uuid, trust: Arc<Mutex<TrustStore>>) -> (Transport, DeviceIdentity) {
    let ca = RoomCa::derive(room).unwrap();
    let device = DeviceIdentity::generate(uuid).unwrap();
    let transport = Transport::new(
        &ca,
        &device,
        trust,
        "127.0.0.1:0".parse().unwrap(),
        64 * 1024,
    )
    .unwrap();
    (transport, device)
}

#[tokio::test]
async fn two_nodes_exchange_text() {
    let (a, _) = build(
        "test-room",
        Uuid::new_v4(),
        Arc::new(Mutex::new(TrustStore::new())),
    );
    let (b, _) = build(
        "test-room",
        Uuid::new_v4(),
        Arc::new(Mutex::new(TrustStore::new())),
    );

    let b_addr = b.local_addr().unwrap();
    let b_uuid = b.local_uuid();
    let (client, server) = tokio::join!(a.connect(b_addr, b_uuid), b.accept());
    let client = client.unwrap();
    let server = server.unwrap();

    let event = ClipboardEvent::new(
        a.local_uuid(),
        Uuid::new_v4(),
        1,
        1,
        EventKind::Text,
        [0; 32],
        5,
    );
    client.send_text(&event, b"hello").await.unwrap();

    let (received, payload) = server.recv_text().await.unwrap();
    assert_eq!(received.event_id, event.event_id);
    assert_eq!(received.origin, a.local_uuid());
    assert_eq!(payload, b"hello");
}

#[tokio::test]
async fn different_room_key_fails_handshake() {
    let (a, _) = build(
        "room-a",
        Uuid::new_v4(),
        Arc::new(Mutex::new(TrustStore::new())),
    );
    let (b, _) = build(
        "room-b",
        Uuid::new_v4(),
        Arc::new(Mutex::new(TrustStore::new())),
    );

    let (client, server) = tokio::join!(
        tokio::time::timeout(
            Duration::from_secs(5),
            a.connect(b.local_addr().unwrap(), b.local_uuid()),
        ),
        tokio::time::timeout(Duration::from_secs(5), b.accept()),
    );

    assert!(client.is_err() || client.unwrap().is_err());
    assert!(server.is_err() || server.unwrap().is_err());
}

#[tokio::test]
async fn changed_device_key_is_rejected() {
    let b_uuid = Uuid::new_v4();
    let trust_b = Arc::new(Mutex::new(TrustStore::new()));
    let (b, b_device) = build("test-room", b_uuid, trust_b.clone());
    trust_b
        .lock()
        .unwrap()
        .check_and_record(b_uuid, b_device.public_key_hash());

    let (fake, _) = build("test-room", b_uuid, Arc::new(Mutex::new(TrustStore::new())));

    let (client, server) = tokio::join!(fake.connect(b.local_addr().unwrap(), b_uuid), b.accept());
    assert!(server.is_err());
    if let Ok(connection) = client {
        connection.close();
    }
}
