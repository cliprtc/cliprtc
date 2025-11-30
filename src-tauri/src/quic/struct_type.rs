use std::{sync::Arc, time::Instant};

use quinn::{
    rustls::{
        self,
        client::danger::{ServerCertVerified, ServerCertVerifier},
        crypto::CryptoProvider,
        pki_types::{CertificateDer, ServerName, UnixTime},
        server::danger::{ClientCertVerified, ClientCertVerifier},
    },
    Connection, RecvStream,
};
use serde::{Deserialize, Serialize};
use tauri_plugin_log::log;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::{
    clipboard::struct_type::EClipboardKind,
    utils::{constant::ALLOW_FINGERPRINT, hash},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayloadMeta {
    pub kind: EClipboardKind,
    pub source_id: String,
    pub seq: u64,
    pub name: String,
    pub total_size: u64,
}
pub struct QuicTransfer;

impl QuicTransfer {
    pub async fn send_payload<R: AsyncRead + Unpin>(
        conn: &Connection,
        meta: &PayloadMeta,
        mut content: R,
    ) -> anyhow::Result<()> {
        let mut send = conn.open_uni().await?;

        log::info!("Start send");
        let start = Instant::now();

        let meta_json = serde_json::to_vec(meta)?;
        let meta_len = meta_json.len() as u32;
        send.write_u32(meta_len).await?;
        send.write_all(&meta_json).await?;

        tokio::io::copy(&mut content, &mut send).await?;

        send.finish()?;
        let elapsed = start.elapsed();
        log::info!("Send in {} s", elapsed.as_secs());

        Ok(())
    }

    pub async fn recv_payload(conn: &Connection) -> anyhow::Result<(PayloadMeta, RecvStream)> {
        let mut recv = conn.accept_uni().await?;

        let meta_len = recv.read_u32().await? as usize;
        let mut meta_buf = vec![0u8; meta_len];
        recv.read_exact(&mut meta_buf).await?;
        let meta: PayloadMeta = serde_json::from_slice(&meta_buf)?;

        Ok((meta, recv))
    }
}

#[derive(Debug)]
pub struct ClientFingerprintVerifier {
    crypto: Arc<CryptoProvider>,
}

impl ClientFingerprintVerifier {
    pub fn new(crypto: Arc<CryptoProvider>) -> Arc<Self> {
        Arc::new(Self { crypto })
    }
}

impl ServerCertVerifier for ClientFingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint = hash::hash(&end_entity);

        if ALLOW_FINGERPRINT.contains(&fingerprint) {
            return Ok(ServerCertVerified::assertion());
        }

        Err(rustls::Error::General(format!(
            "server certificate fingerprint mismatch: got {}",
            fingerprint
        )))
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.crypto
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[derive(Debug)]
pub struct ServerFingerprintVerifier {
    crypto: Arc<CryptoProvider>,
}

impl ServerFingerprintVerifier {
    pub fn new(crypto: Arc<CryptoProvider>) -> Arc<Self> {
        Arc::new(Self { crypto })
    }
}

impl ClientCertVerifier for ServerFingerprintVerifier {
    fn offer_client_auth(&self) -> bool {
        true // 服务器要求客户端提供证书
    }
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        let fingerprint = hash::hash(&end_entity);

        if ALLOW_FINGERPRINT.contains(&fingerprint) {
            return Ok(ClientCertVerified::assertion());
        }

        Err(rustls::Error::General(format!(
            "client certificate fingerprint mismatch: got {}",
            fingerprint
        )))
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.crypto
            .signature_verification_algorithms
            .supported_schemes()
    }
}
