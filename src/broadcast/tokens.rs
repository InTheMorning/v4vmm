//! Token-file storage for ADR 0059 broadcast events.
//!
//! The relay returns a broadcaster token once. The database stores only the
//! path to the token file; this module owns the file write and read boundary.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

#[cfg(unix)]
const DIRECTORY_MODE: u32 = 0o700;
#[cfg(unix)]
const FILE_MODE: u32 = 0o600;
const TEMP_ATTEMPTS: usize = 100;

/// Return the default directory for broadcast token files.
///
/// # Errors
///
/// Returns an error if the application config directory cannot be resolved.
pub fn default_token_directory() -> Result<PathBuf> {
    let config_file = crate::config::config_path()?;
    let config_dir = config_file
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent directory"))?;
    Ok(config_dir.join("broadcast").join("tokens"))
}

/// Return the default token file path for one event identifier.
///
/// # Errors
///
/// Returns an error if the event identifier is empty or the config directory
/// cannot be resolved.
pub fn token_path_for_event(event_id: &str) -> Result<PathBuf> {
    token_path_in_directory(&default_token_directory()?, event_id)
}

/// Return the token file path for one event in a specific directory.
///
/// # Errors
///
/// Returns an error if the event identifier is empty.
pub fn token_path_in_directory(directory: &Path, event_id: &str) -> Result<PathBuf> {
    let event_id = event_id.trim();
    anyhow::ensure!(!event_id.is_empty(), "broadcast event_id cannot be empty");

    let mut hasher = Sha256::new();
    hasher.update(event_id.as_bytes());
    let digest = hasher.finalize();
    let file_name = format!("{digest:x}.token");
    Ok(directory.join(file_name))
}

/// Write a broadcaster token to a mode-restricted file.
///
/// The parent directory is created with mode `0700` on Unix. The token is
/// written to a temporary file with mode `0600`, then renamed into place.
///
/// # Errors
///
/// Returns an error if the token is empty or if any filesystem operation fails.
pub fn write_token_file(path: &Path, token: &str) -> Result<()> {
    anyhow::ensure!(!token.is_empty(), "broadcast token cannot be empty");
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("broadcast token path has no parent directory"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("broadcast token path has no file name"))?;

    fs::create_dir_all(parent)
        .with_context(|| format!("create broadcast token directory {}", parent.display()))?;
    restrict_directory(parent)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("read system time for broadcast token file")?
        .as_nanos();
    for attempt in 0..TEMP_ATTEMPTS {
        let temp_path = parent.join(format!(
            ".{file_name}.{}.{}.{}.tmp",
            std::process::id(),
            nonce,
            attempt
        ));
        let mut file = match create_new_token_file(&temp_path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "create temporary broadcast token file {}",
                        temp_path.display()
                    )
                });
            }
        };
        let write_result = write_token_contents(&mut file, &temp_path, token);
        drop(file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
        restrict_file(&temp_path)?;
        if let Err(error) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            return Err(error)
                .with_context(|| format!("rename broadcast token file {}", path.display()));
        }
        return Ok(());
    }

    anyhow::bail!("could not create temporary broadcast token file")
}

/// Read a broadcaster token from disk.
///
/// # Errors
///
/// Returns an error if the token file cannot be read.
pub fn read_token_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("read broadcast token file {}", path.display()))
}

fn create_new_token_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(FILE_MODE);
    options.open(path)
}

fn write_token_contents(file: &mut File, path: &Path, token: &str) -> Result<()> {
    file.write_all(token.as_bytes())
        .with_context(|| format!("write broadcast token file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync broadcast token file {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(DIRECTORY_MODE))
        .with_context(|| format!("set broadcast token directory mode {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn restrict_file(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(FILE_MODE))
        .with_context(|| format!("set broadcast token file mode {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_file(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn file_mode(path: &Path) -> Result<u32> {
        Ok(fs::metadata(path)
            .with_context(|| format!("read mode for {}", path.display()))?
            .permissions()
            .mode()
            & 0o777)
    }

    #[cfg(unix)]
    #[test]
    fn write_token_file_sets_file_and_directory_modes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp
            .path()
            .join("broadcast")
            .join("tokens")
            .join("event.token");

        write_token_file(&path, "token-one")?;

        assert_eq!(
            file_mode(path.parent().context("token path parent")?)?,
            DIRECTORY_MODE,
            "broadcast token directory should be owner-only"
        );
        assert_eq!(
            file_mode(&path)?,
            FILE_MODE,
            "broadcast token file should be owner-read/write only"
        );
        Ok(())
    }

    #[test]
    fn write_token_file_replaces_existing_content() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp
            .path()
            .join("broadcast")
            .join("tokens")
            .join("event.token");

        write_token_file(&path, "token-one")?;
        write_token_file(&path, "token-two")?;

        assert_eq!(
            read_token_file(&path)?,
            "token-two",
            "rewrite should replace token file content"
        );
        Ok(())
    }
}
