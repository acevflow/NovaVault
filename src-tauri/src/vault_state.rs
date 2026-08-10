use std::path::PathBuf;
use std::sync::Mutex;

pub struct VaultState {
    pub vault: Mutex<Option<OpenVault>>,
}

pub struct OpenVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}