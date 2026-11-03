//! Atomic file replacement.
//!
//! A vault is never written in place: a crash or power loss halfway through
//! would leave a file that opens with neither the old nor the new contents.
//! Instead the new contents go to a temporary file in the same directory,
//! which is flushed to disk and then renamed over the target. A rename
//! within one directory replaces the target atomically on Windows, macOS
//! and Linux, so the vault path always holds either the complete old file
//! or the complete new one.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Atomically replaces (or creates) the file at `path` with `bytes`.
///
/// On Unix the file is created readable and writable by its owner only.
///
/// # Errors
///
/// Any I/O error while creating, writing, flushing or renaming the
/// temporary file. The temporary file is removed on failure and the
/// existing file at `path`, if any, is left untouched.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "vault path has no file name")
    })?;
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };

    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".{suffix}.tmp"));
    let temp = dir.join(temp_name);

    let result = write_and_rename(&temp, path, bytes);
    if result.is_err() {
        // Best effort: the temporary file holds only ciphertext, and the
        // original error is the one worth reporting.
        let _ = fs::remove_file(&temp);
    }
    result?;
    sync_dir(dir)
}

fn write_and_rename(temp: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    // `create_new` refuses to follow or reuse an existing path, so a file
    // planted at the temporary name cannot capture the vault contents.
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(temp)?;
    file.write_all(bytes)?;
    // Flush the data before the rename makes it visible under the real
    // name; otherwise a crash could leave a renamed but empty file.
    file.sync_all()?;
    drop(file);
    fs::rename(temp, path)
}

/// Makes the rename itself durable. Only needed (and possible) on Unix,
/// where a directory can be opened and synced.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempDir;

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn creates_and_replaces_without_leftovers() {
        let dir = TempDir::new();
        let path = dir.path().join("test.nvault");

        write_atomic(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        write_atomic(&path, b"second, longer contents").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second, longer contents");
        write_atomic(&path, b"").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"");

        assert_eq!(entries(dir.path()), ["test.nvault"]);
    }

    #[test]
    fn failure_leaves_original_and_no_temp_file() {
        let dir = TempDir::new();
        let path = dir.path().join("test.nvault");
        write_atomic(&path, b"original").unwrap();

        // Renaming a file over a directory fails on every platform.
        let blocked = dir.path().join("blocked");
        fs::create_dir(&blocked).unwrap();
        assert!(write_atomic(&blocked, b"new").is_err());

        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(entries(dir.path()), ["blocked", "test.nvault"]);
    }

    #[test]
    fn missing_directory_is_an_error() {
        let dir = TempDir::new();
        let path = dir.path().join("missing").join("test.nvault");
        assert_eq!(
            write_atomic(&path, b"x").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[cfg(unix)]
    #[test]
    fn file_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new();
        let path = dir.path().join("test.nvault");
        write_atomic(&path, b"x").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
