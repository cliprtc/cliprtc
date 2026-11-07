use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use quinn::{
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
        server::WebPkiClientVerifier,
        RootCertStore,
    },
    ClientConfig, Connection, Endpoint, ServerConfig,
};
use uuid::Uuid;

use crate::error::{CoreError, Result};
use crate::identity::{
    extract_device_uuid, extract_public_key_hash, DeviceIdentity, RoomCa, TrustDecision, TrustStore,
};
use crate::protocol::{decode_event, encode_event, ClipboardEvent};

const ALPN: &[u8] = b"cliprtc/2";
const SERVER_NAME: &str = "cliprtc.local";
const MAX_EVENT_BYTES: usize = 5 * 1024 * 1024;

pub struct Transport {
    endpoint: Endpoint,
    client_config: ClientConfig,
    trust: Arc<Mutex<TrustStore>>,
    local_uuid: Uuid,
    max_meta_len: usize,
}

#[derive(Clone)]
pub struct PeerConnection {
    connection: Connection,
    local_uuid: Uuid,
    peer_uuid: Uuid,
    max_meta_len: usize,
}

impl Transport {
    pub fn new(
        ca: &RoomCa,
        device: &DeviceIdentity,
        trust: Arc<Mutex<TrustStore>>,
        bind: SocketAddr,
        max_meta_len: usize,
    ) -> Result<Self> {
        let device_cert = ca.issue_device_cert(device)?;

        let mut roots = RootCertStore::empty();
        roots
            .add(ca.cert_der().clone())
            .map_err(|e| CoreError::Transport(format!("root cert failed: {e}")))?;

        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots.clone())
            .with_client_auth_cert(
                vec![device_cert.cert_der.clone()],
                PrivateKeyDer::Pkcs8(device_cert.key_der.clone_key()),
            )
            .map_err(|e| CoreError::Transport(format!("client tls failed: {e}")))?;
        client_crypto.alpn_protocols = vec![ALPN.to_vec()];
        let client_config = ClientConfig::new(Arc::new(
            QuicClientConfig::try_from(client_crypto)
                .map_err(|e| CoreError::Transport(format!("client quic failed: {e}")))?,
        ));

        let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| CoreError::Transport(format!("client verifier failed: {e}")))?;
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![device_cert.cert_der],
                PrivateKeyDer::Pkcs8(device_cert.key_der),
            )
            .map_err(|e| CoreError::Transport(format!("server tls failed: {e}")))?;
        server_crypto.alpn_protocols = vec![ALPN.to_vec()];
        let server_config = ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto)
                .map_err(|e| CoreError::Transport(format!("server quic failed: {e}")))?,
        ));

        let endpoint = Endpoint::server(server_config, bind)?;
        Ok(Self {
            endpoint,
            client_config,
            trust,
            local_uuid: device.uuid,
            max_meta_len,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(CoreError::Io)
    }

    pub fn local_uuid(&self) -> Uuid {
        self.local_uuid
    }

    pub async fn connect(&self, addr: SocketAddr, expected_uuid: Uuid) -> Result<PeerConnection> {
        let connecting = self
            .endpoint
            .connect_with(self.client_config.clone(), addr, SERVER_NAME)
            .map_err(|e| CoreError::Transport(format!("connect failed: {e}")))?;
        let connection = connecting
            .await
            .map_err(|e| CoreError::Transport(format!("handshake failed: {e}")))?;

        let (peer_uuid, peer_hash) = authenticate_peer(&connection)?;
        if peer_uuid != expected_uuid {
            connection.close(0u32.into(), b"unexpected peer");
            return Err(CoreError::Auth(format!(
                "expected peer {expected_uuid}, got {peer_uuid}"
            )));
        }
        record_trust(&self.trust, peer_uuid, peer_hash)?;

        Ok(PeerConnection {
            connection,
            local_uuid: self.local_uuid,
            peer_uuid,
            max_meta_len: self.max_meta_len,
        })
    }

    pub async fn accept(&self) -> Result<PeerConnection> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| CoreError::Transport("endpoint closed".into()))?;
        let connection = incoming
            .await
            .map_err(|e| CoreError::Transport(format!("accept failed: {e}")))?;

        let (peer_uuid, peer_hash) = authenticate_peer(&connection)?;
        record_trust(&self.trust, peer_uuid, peer_hash)?;

        Ok(PeerConnection {
            connection,
            local_uuid: self.local_uuid,
            peer_uuid,
            max_meta_len: self.max_meta_len,
        })
    }

    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"shutdown");
    }
}

impl PeerConnection {
    pub fn peer_uuid(&self) -> Uuid {
        self.peer_uuid
    }

    pub async fn send_text(&self, event: &ClipboardEvent, text: &[u8]) -> Result<()> {
        if event.origin != self.local_uuid {
            return Err(CoreError::Auth(
                "event origin does not match the local device".into(),
            ));
        }
        if text.len() as u64 != event.size {
            return Err(CoreError::Protocol("text size does not match event".into()));
        }

        let meta = encode_event(event, self.max_meta_len)?;
        let mut send = self
            .connection
            .open_uni()
            .await
            .map_err(|e| CoreError::Transport(format!("open stream failed: {e}")))?;
        send.write_all(&meta)
            .await
            .map_err(|e| CoreError::Transport(format!("write meta failed: {e}")))?;
        send.write_all(text)
            .await
            .map_err(|e| CoreError::Transport(format!("write body failed: {e}")))?;
        send.finish()
            .map_err(|e| CoreError::Transport(format!("finish stream failed: {e}")))?;
        Ok(())
    }

    pub async fn recv_text(&self) -> Result<(ClipboardEvent, Vec<u8>)> {
        let mut recv = self
            .connection
            .accept_uni()
            .await
            .map_err(|e| CoreError::Transport(format!("accept stream failed: {e}")))?;
        let buf = recv
            .read_to_end(MAX_EVENT_BYTES)
            .await
            .map_err(|e| CoreError::Transport(format!("read stream failed: {e}")))?;

        if buf.len() < 4 {
            return Err(CoreError::Protocol("event header truncated".into()));
        }
        let meta_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if meta_len > self.max_meta_len {
            return Err(CoreError::Protocol(format!(
                "meta too large: {meta_len} > {}",
                self.max_meta_len
            )));
        }
        let meta_end = 4 + meta_len;
        if buf.len() < meta_end {
            return Err(CoreError::Protocol("event meta truncated".into()));
        }

        let event = decode_event(&buf[..meta_end], self.max_meta_len)?;
        if event.origin != self.peer_uuid {
            return Err(CoreError::Auth(format!(
                "event origin {} does not match authenticated peer {}",
                event.origin, self.peer_uuid
            )));
        }
        let payload = buf[meta_end..].to_vec();
        if payload.len() as u64 != event.size {
            return Err(CoreError::Protocol(format!(
                "payload size mismatch: got {}, expected {}",
                payload.len(),
                event.size
            )));
        }
        Ok((event, payload))
    }

    pub fn close(&self) {
        self.connection.close(0u32.into(), b"bye");
    }
}

fn authenticate_peer(connection: &Connection) -> Result<(Uuid, [u8; 32])> {
    let identity = connection
        .peer_identity()
        .ok_or_else(|| CoreError::Auth("peer did not present a certificate".into()))?;
    let certs = identity
        .downcast::<Vec<CertificateDer<'static>>>()
        .map_err(|_| CoreError::Auth("unexpected peer identity type".into()))?;
    let cert = certs
        .first()
        .ok_or_else(|| CoreError::Auth("empty peer certificate chain".into()))?;
    let uuid = extract_device_uuid(cert.as_ref())?;
    let hash = extract_public_key_hash(cert.as_ref())?;
    Ok((uuid, hash))
}

fn record_trust(trust: &Arc<Mutex<TrustStore>>, uuid: Uuid, hash: [u8; 32]) -> Result<()> {
    let mut store = trust
        .lock()
        .map_err(|_| CoreError::Trust("trust store poisoned".into()))?;
    match store.check_and_record(uuid, hash) {
        TrustDecision::New | TrustDecision::Known => Ok(()),
        TrustDecision::Changed => Err(CoreError::Auth(format!(
            "peer {uuid} changed its device key"
        ))),
        TrustDecision::Blocked => Err(CoreError::Auth(format!("peer {uuid} is blocked"))),
    }
}
