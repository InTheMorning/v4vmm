//! Retained artifact identity and contained staging reuse (ADRs 0056, 0064, 0066).

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use super::{create_staging_dir, validate_downloaded_size, DownloadedTrack, SelectedEnclosure};
use crate::audio_format::{AudioFormat, ConversionOutcome};
use crate::config::DownloadConfig;
use crate::library_path::LibraryRelativePath;

#[derive(Debug)]
pub(crate) struct OwnedStaging {
    path: PathBuf,
    music_dir: PathBuf,
    #[cfg(unix)]
    identity: (u64, u64),
}

impl OwnedStaging {
    pub(super) fn capture(music_dir: &Path, path: PathBuf) -> Result<Self> {
        let metadata = fs::symlink_metadata(&path)?;
        anyhow::ensure!(
            metadata.is_dir(),
            "owned staging is not a directory: {}",
            path.display()
        );
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            path,
            music_dir: music_dir.to_path_buf(),
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
        })
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    pub(super) fn cleanup(&self) -> Result<()> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("inspect owned staging {}", self.path.display()))
            }
        };
        validate_destination(&self.music_dir, &self.path)?;
        anyhow::ensure!(
            metadata.is_dir(),
            "App kept changed staging directory {}",
            self.path.display()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            anyhow::ensure!(
                (metadata.dev(), metadata.ino()) == self.identity,
                "App kept replaced staging directory {}",
                self.path.display()
            );
        }
        fs::remove_dir_all(&self.path).with_context(|| {
            format!(
                "App could not remove owned download staging {}",
                self.path.display()
            )
        })
    }
}

#[derive(Debug)]
pub(crate) struct RetainedArtifact {
    music_dir: PathBuf,
    path: PathBuf,
    canonical: PathBuf,
    digest: [u8; 32],
    bytes: Option<i64>,
    #[cfg(unix)]
    identity: (u64, u64),
}

impl RetainedArtifact {
    pub(crate) fn capture(music_dir: &Path, path: &Path, bytes: Option<i64>) -> Result<Self> {
        LibraryRelativePath::from_absolute(music_dir, path)?;
        validate_destination(music_dir, path)?;
        let metadata = fs::symlink_metadata(path)
            .with_context(|| format!("inspect retained input {}", path.display()))?;
        anyhow::ensure!(
            metadata.is_file(),
            "retained input is not a regular file: {}",
            path.display()
        );
        validate_downloaded_size(path, bytes)?;
        AudioFormat::detect_from_file(path)?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            music_dir: music_dir.to_path_buf(),
            path: path.to_path_buf(),
            canonical: path.canonicalize()?,
            digest: fingerprint(path)?,
            bytes,
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn music_dir(&self) -> &Path {
        &self.music_dir
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let current = Self::capture(&self.music_dir, &self.path, self.bytes)?;
        anyhow::ensure!(
            self.canonical == current.canonical && self.digest == current.digest,
            "retained input moved or changed: {}",
            self.path.display()
        );
        #[cfg(unix)]
        anyhow::ensure!(
            self.identity == current.identity,
            "retained input was replaced: {}",
            self.path.display()
        );
        Ok(())
    }
}

fn fingerprint(path: &Path) -> Result<[u8; 32]> {
    let mut file =
        File::open(path).with_context(|| format!("read retained input {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    Ok(digest.finalize().into())
}

pub(crate) fn validate_destination(music_dir: &Path, path: &Path) -> Result<()> {
    LibraryRelativePath::from_absolute(music_dir, path)?;
    let root = music_dir
        .canonicalize()
        .context("resolve music storage for retained input")?;
    let mut ancestor = path;
    while !ancestor.exists() {
        // A dangling symlink must not be treated as an absent destination.
        anyhow::ensure!(
            fs::symlink_metadata(ancestor).is_err(),
            "unusable retained destination: {}",
            ancestor.display()
        );
        ancestor = ancestor
            .parent()
            .context("find retained destination parent")?;
    }
    anyhow::ensure!(
        ancestor.canonicalize()?.starts_with(root),
        "retained destination leaves music storage: {}",
        path.display()
    );
    Ok(())
}

/// Copy a validated library WAV into exclusively owned staging before conversion.
pub(crate) fn stage_existing(
    cfg: &DownloadConfig,
    artifact: &RetainedArtifact,
    enclosure: SelectedEnclosure,
) -> Result<DownloadedTrack> {
    artifact.validate()?;
    let staging_dir = create_staging_dir(cfg)?;
    let path = staging_dir.path().join("input.wav");
    let mut downloaded = DownloadedTrack {
        path: path.clone(),
        final_path: artifact.path().to_path_buf(),
        enclosure,
        detected_format: AudioFormat::Wav,
        format_warning: None,
        conversion: ConversionOutcome::NotRequired,
        source_warning: None,
        input: None,
        staging_dir: Some(staging_dir),
    };
    fs::copy(artifact.path(), &path)
        .with_context(|| format!("stage retained WAV {}", artifact.path().display()))?;
    downloaded.input = Some(RetainedArtifact::capture(&cfg.music_dir, &path, None)?);
    downloaded.convert(cfg);
    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn adr_0066_retained_artifact_checks_size_bytes_identity_and_containment() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input.wav");
        let wav = b"RIFF\x24\0\0\0WAVEfmt ";
        fs::write(&path, wav).unwrap();
        assert!(RetainedArtifact::capture(temp.path(), &path, Some(100)).is_err());
        let input = RetainedArtifact::capture(temp.path(), &path, Some(16)).unwrap();
        input.validate().unwrap();
        fs::rename(&path, temp.path().join("old.wav")).unwrap();
        fs::write(&path, wav).unwrap();
        assert!(
            input.validate().is_err(),
            "same bytes in a replacement inode are not the retained file"
        );
        fs::remove_file(&path).unwrap();
        symlink(temp.path().join("old.wav"), &path).unwrap();
        assert!(RetainedArtifact::capture(temp.path(), &path, None).is_err());
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), temp.path().join("moved")).unwrap();
        assert!(validate_destination(temp.path(), &temp.path().join("moved/output.flac")).is_err());
    }

    #[test]
    fn adr_0066_staging_cleanup_refuses_replaced_directory_and_reports_its_path() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("owned");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("input.wav"), b"owned input").unwrap();
        let owned = OwnedStaging::capture(temp.path(), directory.clone()).unwrap();
        fs::rename(&directory, temp.path().join("moved-owned")).unwrap();
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("music.wav"), b"existing music").unwrap();
        let error = owned.cleanup().unwrap_err().to_string();
        assert!(error.contains(&directory.display().to_string()));
        assert_eq!(
            fs::read(directory.join("music.wav")).unwrap(),
            b"existing music"
        );
        assert!(temp.path().join("moved-owned/input.wav").exists());
        fs::rename(&directory, temp.path().join("preserved-other")).unwrap();
        fs::rename(temp.path().join("moved-owned"), &directory).unwrap();
        owned.cleanup().unwrap();
        assert!(!directory.exists());
        assert!(temp.path().join("preserved-other/music.wav").exists());
    }
}
