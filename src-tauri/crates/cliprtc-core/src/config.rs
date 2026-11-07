use crate::error::{CoreError, Result};

pub const DEFAULT_MAX_META_LEN: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub room_key: String,
    pub max_meta_len: usize,
}

impl CoreConfig {
    pub fn new(room_key: impl Into<String>) -> Result<Self> {
        let room_key = room_key.into();
        if room_key.trim().is_empty() {
            return Err(CoreError::Config("room key must not be empty".into()));
        }
        Ok(Self {
            room_key,
            max_meta_len: DEFAULT_MAX_META_LEN,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_room_key() {
        assert!(CoreConfig::new("   ").is_err());
    }

    #[test]
    fn uses_default_meta_limit() {
        let cfg = CoreConfig::new("room").unwrap();
        assert_eq!(cfg.max_meta_len, DEFAULT_MAX_META_LEN);
    }
}
