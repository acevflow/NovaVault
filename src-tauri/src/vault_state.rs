use std::path::PathBuf;
use std::sync::Mutex;

pub struct VaultState {
    pub vault: Mutex<Option<OpenVault>>,
    pub pending_unlock: Mutex<Option<PendingUnlock>>,
}

pub struct OpenVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

pub struct PendingUnlock {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}
