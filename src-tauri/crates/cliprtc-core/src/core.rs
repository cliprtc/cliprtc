use uuid::Uuid;

use crate::config::CoreConfig;
use crate::error::Result;
use crate::identity::{DeviceCert, DeviceIdentity, RoomCa, TrustStore};
use crate::protocol::LamportClock;

pub struct Core {
    pub device: DeviceIdentity,
    pub room_ca: RoomCa,
    pub trust: TrustStore,
    pub clock: LamportClock,
}

impl Core {
    pub fn new(config: CoreConfig, uuid: Uuid) -> Result<Self> {
        let device = DeviceIdentity::generate(uuid)?;
        let room_ca = RoomCa::derive(&config.room_key)?;
        Ok(Self {
            device,
            room_ca,
            trust: TrustStore::new(),
            clock: LamportClock::new(),
        })
    }

    pub fn device_cert(&self) -> Result<DeviceCert> {
        self.room_ca.issue_device_cert(&self.device)
    }

    pub fn trust(&self) -> &TrustStore {
        &self.trust
    }

    pub fn trust_mut(&mut self) -> &mut TrustStore {
        &mut self.trust
    }
}
