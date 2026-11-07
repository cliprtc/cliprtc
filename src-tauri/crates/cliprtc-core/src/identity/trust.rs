use std::collections::HashMap;
use std::time::SystemTime;

use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTrust {
    pub public_key_hash: [u8; 32],
    pub first_seen: SystemTime,
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustDecision {
    New,
    Known,
    Changed,
    Blocked,
}

#[derive(Debug, Default)]
pub struct TrustStore {
    peers: HashMap<Uuid, PeerTrust>,
}

impl TrustStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_and_record(&mut self, uuid: Uuid, public_key_hash: [u8; 32]) -> TrustDecision {
        if let Some(existing) = self.peers.get_mut(&uuid) {
            if existing.blocked {
                return TrustDecision::Blocked;
            }
            if existing.public_key_hash == public_key_hash {
                return TrustDecision::Known;
            }
            return TrustDecision::Changed;
        }

        self.peers.insert(
            uuid,
            PeerTrust {
                public_key_hash,
                first_seen: SystemTime::now(),
                blocked: false,
            },
        );
        TrustDecision::New
    }

    pub fn block(&mut self, uuid: Uuid) {
        if let Some(peer) = self.peers.get_mut(&uuid) {
            peer.blocked = true;
        }
    }

    pub fn get(&self, uuid: &Uuid) -> Option<&PeerTrust> {
        self.peers.get(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_seen_is_new_then_known() {
        let mut store = TrustStore::new();
        let id = Uuid::new_v4();
        assert_eq!(store.check_and_record(id, [1; 32]), TrustDecision::New);
        assert_eq!(store.check_and_record(id, [1; 32]), TrustDecision::Known);
    }

    #[test]
    fn changed_key_is_rejected() {
        let mut store = TrustStore::new();
        let id = Uuid::new_v4();
        store.check_and_record(id, [1; 32]);
        assert_eq!(store.check_and_record(id, [2; 32]), TrustDecision::Changed);
    }

    #[test]
    fn blocked_peer_is_rejected() {
        let mut store = TrustStore::new();
        let id = Uuid::new_v4();
        store.check_and_record(id, [1; 32]);
        store.block(id);
        assert_eq!(store.check_and_record(id, [1; 32]), TrustDecision::Blocked);
    }
}
