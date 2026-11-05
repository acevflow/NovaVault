//! Atomic file replacement.
//!
//! A vault is never written in place: a crash or power loss halfway through
//! would leave a file that opens with neither the old nor the new contents.
//! Instead the new contents go to a temporary file in the same directory,
//! which is flushed to disk and then renamed over the target. A rename
//! within one directory replaces the target atomically on Windows, macOS
//! and Linux, so the vault path always holds either the complete old file
//! or the complete new one.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
    let mut file = AtomicFile::create(path)?;
    file.write_all(bytes)?;
    file.commit()
}

/// A file written in a temporary location and moved to its final path only
/// when [`AtomicFile::commit`] succeeds, for contents too large to hold in
/// memory (attachments).
///
/// Dropping an uncommitted `AtomicFile` deletes the temporary file, so an
/// error or a cancellation halfway through never leaves a partial file at
/// the target path, and never leaves a partially written temporary file
/// behind either. That matters for exports: they hold plaintext that has
/// not all been authenticated yet.
pub(crate) struct AtomicFile {
    /// The open temporary file; `None` once committed.
    file: Option<File>,
    temp: PathBuf,
    target: PathBuf,
    dir: PathBuf,
}

impl AtomicFile {
    /// Creates a new temporary file next to `path`.
    ///
    /// On Unix it is readable and writable by its owner only.
    ///
    /// # Errors
    ///
    /// Any I/O error while creating the temporary file, or
    /// [`io::ErrorKind::InvalidInput`] if `path` has no file name.
    pub(crate) fn create(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
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

        let mut options = OpenOptions::new();
        // `create_new` refuses to follow or reuse an existing path, so a
        // file planted at the temporary name cannot capture the contents.
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&temp)?;
        Ok(Self {
            file: Some(file),
            temp,
            target: path.to_path_buf(),
            dir: dir.to_path_buf(),
        })
    }

    /// Flushes the contents to disk and renames the temporary file over the
    /// target path, replacing any file there.
    ///
    /// # Errors
    ///
    /// Any I/O error while flushing or renaming. The temporary file is then
    /// removed and the target path is left untouched.
    pub(crate) fn commit(mut self) -> io::Result<()> {
        let file = self.file.take().ok_or_else(closed)?;
        // Flush the data before the rename makes it visible under the real
        // name; otherwise a crash could leave a renamed but empty file.
        let result = file.sync_all().and_then(|()| {
            drop(file);
            fs::rename(&self.temp, &self.target)
        });
        if result.is_err() {
            // Best effort: the original error is the one worth reporting.
            let _ = fs::remove_file(&self.temp);
        }
        result?;
        sync_dir(&self.dir)
    }
}

impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.as_mut().ok_or_else(closed)?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().ok_or_else(closed)?.flush()
    }
}

impl Drop for AtomicFile {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            // Close before deleting: Windows cannot always delete an open
            // file.
            drop(file);
            let _ = fs::remove_file(&self.temp);
        }
    }
}

fn closed() -> io::Error {
    io::Error::other("file already committed")
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

    #[test]
    fn uncommitted_files_leave_nothing_behind() {
        let dir = TempDir::new();
        let path = dir.path().join("export.bin");
        fs::write(&path, b"previous").unwrap();
        let mut file = AtomicFile::create(&path).unwrap();
        file.write_all(b"partial").unwrap();
        drop(file);
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        assert_eq!(entries(dir.path()), ["export.bin"]);

        let mut file = AtomicFile::create(&path).unwrap();
        file.write_all(b"complete").unwrap();
        file.commit().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete");
        assert_eq!(entries(dir.path()), ["export.bin"]);
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
