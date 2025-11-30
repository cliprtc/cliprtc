use std::{
    io::Cursor,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use quinn::Endpoint;
use tauri_plugin_log::log;
use tokio::fs::File;

use crate::{
    clipboard::struct_type::{ClipboardInfo, EClipboardKind},
    quic::struct_type::{PayloadMeta, QuicTransfer},
    utils::constant::{CLIPBOARD_INFO, DEVICES},
};

pub fn start(client: Endpoint) {
    tokio::spawn(async move {
        let mut rx = CLIPBOARD_INFO.rx.write().await;
        while let Some(data) = rx.recv().await {
            let data = Arc::new(data);
            handle_request(client.clone(), data).await;
        }
    });
}

async fn handle_request(client: Endpoint, data: Arc<ClipboardInfo>) {
    for device in DEVICES.iter() {
        let client = client.clone();
        let data = Arc::clone(&data);
        tokio::spawn(async move {
            let ip_addrs = device.addresses.v4.clone();

            if ip_addrs.is_empty() {
                log::warn!("Device addresses is empty, skipping: {}", device.hostname);
                return;
            }

            let ip = ip_addrs.first().unwrap();

            let server_addr = SocketAddr::new(IpAddr::V4(*ip), device.port);
            let connect = client.connect(server_addr, "localhost").unwrap();
            match connect.await {
                Ok(connection) => {
                    match data.kind {
                        EClipboardKind::Text => handle_text(&connection, data).await,
                        EClipboardKind::Image => handle_image(&connection, data).await,
                        EClipboardKind::File => handle_file(&connection, data).await,
                    }
                    connection.closed().await;
                }
                Err(e) => {
                    log::error!(
                        "Failed to connect to server: [{}] Error: {}",
                        server_addr,
                        e
                    );
                }
            };
        });
    }
}

async fn handle_text(connection: &quinn::Connection, data: Arc<ClipboardInfo>) {
    let meta = PayloadMeta {
        kind: data.kind.clone(),
        source_id: data.source_id.clone(),
        seq: data.seq,
        name: "".to_string(),
        total_size: data.content.len() as u64,
    };

    let reader = Cursor::new(data.content.clone());

    if let Err(e) = QuicTransfer::send_payload(&connection, &meta, reader).await {
        log::error!("Failed to send payload: {}", e);
    }
}
async fn handle_image(connection: &quinn::Connection, data: Arc<ClipboardInfo>) {
    let meta = PayloadMeta {
        kind: data.kind.clone(),
        source_id: data.source_id.clone(),
        seq: data.seq,
        name: "".to_string(),
        total_size: data.content.len() as u64,
    };
    let reader = Cursor::new(data.content.clone());

    if let Err(e) = QuicTransfer::send_payload(&connection, &meta, reader).await {
        log::error!("Failed to send payload: {}", e);
    }
}
async fn handle_file(connection: &quinn::Connection, data: Arc<ClipboardInfo>) {
    let path_str = String::from_utf8_lossy(&data.content).to_string();
    let path = PathBuf::from(&path_str);
    if path.is_file() {
        match File::open(&path).await {
            Ok(file) => {
                let file_name_owned = path
                    .file_name()
                    .and_then(|os| os.to_str())
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| "file".to_string());

                let total_size = file.metadata().await.unwrap().len();

                let meta = PayloadMeta {
                    kind: data.kind.clone(),
                    source_id: data.source_id.clone(),
                    seq: data.seq,
                    name: file_name_owned,
                    total_size,
                };
                if let Err(e) = QuicTransfer::send_payload(&connection, &meta, file).await {
                    log::error!("Failed to send payload: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Open file failed: {}", e);
            }
        }
    }
}
