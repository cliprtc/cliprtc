use quinn::rustls::pki_types::CertificateDer;
use tauri::async_runtime::spawn;

use crate::{clipboard, mdns, quic, utils::hash::hash};

pub fn start() {
    clipboard::start();

    spawn(async move {
        let (_endpoint, port, cert) = quic::start();

        start_mdns(port, cert);
    });
}

fn start_mdns(port: u16, cert: CertificateDer<'static>) {
    let fingerprint = hash(&cert);
    let _mdns_handle = mdns::start(port, fingerprint);
}
