use std::path::PathBuf;
use std::sync::Mutex;

pub struct VaultState {
    pub vault: Mutex<Option<OpenVault>>,
    pub pending_unlock: Mutex<Option<PendingUnlock>>,
    pub unlocked_vaults: Mutex<Vec<OpenVault>>,
}

#[derive(Clone)]
pub struct OpenVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone)]
pub struct PendingUnlock {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}
