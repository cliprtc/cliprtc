use mdns_sd::ServiceDaemon;

mod query;
mod register;

use query::query;
use register::register;

pub fn start(port: u16, fingerprint: String) -> ServiceDaemon {
    let mdns = register(port, fingerprint);
    query(&mdns);
    mdns
}
