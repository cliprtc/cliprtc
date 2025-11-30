use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    hash.into()
}

pub fn hash(data: &[u8]) -> String {
    let result = sha256(data);
    hex::encode(result)
}
