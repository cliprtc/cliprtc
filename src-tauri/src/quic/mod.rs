use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use quinn::{
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    rustls::{
        self,
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
    },
    ClientConfig, Endpoint, ServerConfig, TransportConfig,
};

use crate::quic::struct_type::{ClientFingerprintVerifier, ServerFingerprintVerifier};

mod client;
mod server;
mod struct_type;

pub fn start() -> (Endpoint, u16, CertificateDer<'static>) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
    let (endpoint, server_cert) = make_server_endpoint(bind_addr).unwrap();

    client::start(endpoint.clone());
    server::start(endpoint.clone());

    let port = endpoint.local_addr().unwrap().port();

    (endpoint, port, server_cert)
}

pub fn make_server_endpoint(
    bind_addr: SocketAddr,
) -> Result<(Endpoint, CertificateDer<'static>), Box<dyn Error + Send + Sync + 'static>> {
    let self_signed_cert = generate_self_signed();
    let server_self_signed_cert = (self_signed_cert.0.clone(), self_signed_cert.1.clone_key());
    let server_cert = self_signed_cert.0.clone();

    let transport_config = make_transport_config();
    let mut server_config = configure_server(server_self_signed_cert)?;
    let mut client_config = configure_client(self_signed_cert)?;

    client_config.transport_config(transport_config.clone());
    server_config.transport_config(transport_config.clone());

    let mut endpoint = Endpoint::server(server_config, bind_addr)?;
    endpoint.set_default_client_config(client_config);

    Ok((endpoint, server_cert))
}

fn make_transport_config() -> Arc<TransportConfig> {
    let transport = TransportConfig::default();

    let transport = Arc::new(transport);
    transport
}

fn configure_client(
    cert: (CertificateDer<'static>, PrivatePkcs8KeyDer<'static>),
) -> Result<ClientConfig, Box<dyn Error + Send + Sync + 'static>> {
    let (cert_der, key_der) = cert;

    let crypto = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = ClientFingerprintVerifier::new(crypto);

    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(vec![cert_der], key_der.into())?;
    tls_config.alpn_protocols = vec![b"hq-29".to_vec()];
    let quic_config = Arc::new(QuicClientConfig::try_from(tls_config)?);
    let client_config = ClientConfig::new(quic_config);

    Ok(client_config)
}

fn configure_server(
    cert: (CertificateDer<'static>, PrivatePkcs8KeyDer<'static>),
) -> Result<ServerConfig, Box<dyn Error + Send + Sync + 'static>> {
    let (cert_der, key_der) = cert;

    let crypto = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = ServerFingerprintVerifier::new(crypto);

    let mut tls_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(vec![cert_der.clone()], key_der.into())?;
    tls_config.alpn_protocols = vec![b"hq-29".to_vec()];
    let quic_config = Arc::new(QuicServerConfig::try_from(tls_config)?);
    let server_config = ServerConfig::with_crypto(quic_config);

    Ok(server_config)
}

fn generate_self_signed() -> (CertificateDer<'static>, PrivatePkcs8KeyDer<'static>) {
    let cert = rcgen::generate_simple_self_signed(vec![]).unwrap();
    let cert_der = CertificateDer::from(cert.cert);
    let key_der = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());

    (cert_der, key_der)
}
