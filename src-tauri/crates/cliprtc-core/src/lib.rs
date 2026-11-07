pub mod config;
pub mod core;
pub mod error;
pub mod identity;
pub mod protocol;
pub mod transport;

pub use config::CoreConfig;
pub use core::Core;
pub use error::{CoreError, Result};
pub use identity::RoomCa;
pub use protocol::{ClipboardEvent, DedupWindow, EventKind, LamportClock, PROTOCOL_VERSION};
pub use transport::{PeerConnection, Transport};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert_eq!(super::VERSION, "0.1.0");
    }
}
