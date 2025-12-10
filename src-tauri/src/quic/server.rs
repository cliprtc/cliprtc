use bytesize::ByteSize;
use clipboard_rs::{common::RustImage, Clipboard, RustImageData};
use quinn::{Endpoint, RecvStream};
use std::{path::Path, sync::atomic::Ordering, time::Instant};
use tauri_plugin_log::log;
use tokio::{
    fs::{self},
    io::AsyncWriteExt,
};

use crate::{
    clipboard::struct_type::EClipboardKind,
    quic::struct_type::QuicTransfer,
    utils::{
        compress::decompress_path,
        constant::{ALLOW_DEVICE_IDS, APP_TMP_DIR_FILES_CACHE, CLIPBOARD_INFO, UUID},
    },
};

pub fn start(server: Endpoint) {
    tokio::spawn(handle_connection(server));
}

async fn handle_connection(server: Endpoint) {
    while let Some(incoming) = server.accept().await {
        tokio::spawn(async move {
            log::info!("incoming connection: addr={}", incoming.remote_address());

            let connection = match incoming.await {
                Ok(connection) => connection,
                Err(e) => {
                    log::error!("Error accepting incoming connection: {}", e);
                    return;
                }
            };
            handle_request(&connection).await;
        });
    }
}

async fn handle_request(connection: &quinn::Connection) {
    match QuicTransfer::recv_payload(&connection).await {
        Ok((meta, recv)) => {
            let source_id = meta.source_id;
            let seq = meta.seq;
            if source_id == UUID.to_string() {
                return;
            }
            if !ALLOW_DEVICE_IDS.contains(&source_id) {
                return;
            }

            let map = &CLIPBOARD_INFO.remote_seq_map;
            if let Some(last_seq) = map.get(&source_id) {
                if seq <= last_seq.clone() {
                    return;
                }
            }
            map.insert(source_id.clone(), seq);

            CLIPBOARD_INFO.lock.store(true, Ordering::SeqCst);

            let chunk_size = 2 * 1024 * 1024;

            match meta.kind {
                EClipboardKind::Text => {
                    if let Err(e) = handle_text(recv, chunk_size).await {
                        log::error!("Error handling text: {}", e);
                    }
                }
                EClipboardKind::Image => {
                    if let Err(e) = handle_image(recv, chunk_size).await {
                        log::error!("Error handling image: {}", e);
                    }
                }
                EClipboardKind::File => {
                    if let Err(e) = handle_file(meta.name, recv, chunk_size).await {
                        log::error!("Error handling file: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("failed to receive payload: {}", e);
        }
    }
}

async fn handle_text(mut recv: RecvStream, chunk_size: usize) -> anyhow::Result<()> {
    let start = Instant::now();

    let mut buf = Vec::new();
    while let Some(chunk) = recv.read_chunk(chunk_size, true).await? {
        buf.extend_from_slice(&chunk.bytes);
    }

    let elapsed = start.elapsed();
    log::info!(
        "Received Text ({}) in {} ms",
        ByteSize(buf.len() as u64),
        elapsed.as_millis()
    );

    let text = String::from_utf8(buf)?;
    if let Err(e) = CLIPBOARD_INFO.ctx.set_text(text) {
        log::error!("Failed to set clipboard text: {}", e);
    }
    Ok(())
}

async fn handle_image(mut recv: RecvStream, chunk_size: usize) -> anyhow::Result<()> {
    let start = Instant::now();

    let mut buf = Vec::new();
    while let Some(chunk) = recv.read_chunk(chunk_size, true).await? {
        buf.extend_from_slice(&chunk.bytes);
    }

    let elapsed = start.elapsed();
    log::info!(
        "Received Image ({}) in {} s",
        ByteSize(buf.len() as u64),
        elapsed.as_secs()
    );

    if let Ok(image) = RustImageData::from_bytes(&buf) {
        if let Err(e) = CLIPBOARD_INFO.ctx.set_image(image) {
            log::error!("Failed to set clipboard image: {}", e);
        }
    }

    Ok(())
}

async fn handle_file(name: String, mut recv: RecvStream, chunk_size: usize) -> anyhow::Result<()> {
    let mut save_path = APP_TMP_DIR_FILES_CACHE.clone();

    if let Err(e) = fs::create_dir_all(&save_path).await {
        log::error!("Failed to create the directory: {}", e);
    }

    save_path.push(&name);
    match fs::File::create(&save_path).await {
        Ok(mut file) => {
            let mut total_bytes = 0usize;
            let mut buf = vec![0u8; chunk_size];
            let mut max_chunk = 0usize;
            let mut min_chunk = 0usize;
            let start = Instant::now();

            while let Some(chunk) = recv.read(&mut buf).await? {
                if let Err(e) = file.write_all(&buf[..chunk]).await {
                    log::error!("Failed to write chunk: {}", e);
                    break;
                }
                if max_chunk == 0 {
                    max_chunk = chunk;
                    min_chunk = chunk;
                }

                if chunk > max_chunk {
                    max_chunk = chunk;
                }
                if chunk < min_chunk {
                    min_chunk = chunk;
                }
                total_bytes += chunk;
            }

            let elapsed = start.elapsed();
            log::info!("Recevied time: {} s", elapsed.as_secs());

            log::info!(
                "Received {} chunks: min: {}, max: {}",
                ByteSize(total_bytes as u64),
                ByteSize(min_chunk as u64),
                ByteSize(max_chunk as u64)
            );

            log::info!(
                "Received file [{}] ({})",
                name,
                ByteSize(total_bytes as u64)
            );
            let mut extract_dir = APP_TMP_DIR_FILES_CACHE.clone();
            let pack_stem = Path::new(&name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            extract_dir.push(&pack_stem);

            if let Err(e) = fs::create_dir_all(&extract_dir).await {
                log::error!("Failed to create extract dir: {}", e);
            }

            if let Err(e) = decompress_path(&save_path, &extract_dir) {
                log::error!("Failed to decompress pack: {}", e);
            }

            // 遍历解压后的所有文件
            let mut paths = Vec::new();
            let mut dir_entries = fs::read_dir(&extract_dir).await.unwrap();
            while let Ok(Some(entry)) = dir_entries.next_entry().await {
                let path = entry.path();
                paths.push(path.to_string_lossy().to_string());
            }

            if let Err(e) = CLIPBOARD_INFO.ctx.set_files(paths) {
                log::error!("Failed to set clipboard files: {}", e);
            }
        }
        Err(e) => log::error!("Failed to create the file: [{}]: {}", name, e),
    }

    Ok(())
}
