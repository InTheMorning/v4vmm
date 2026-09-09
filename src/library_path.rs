//! Local library path address model for ADR 0064.

#![warn(clippy::pedantic)]

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

/// Stored path for a file under `music_dir`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LibraryRelativePath(String);

impl LibraryRelativePath {
    /// Creates a stored path from an absolute file path under `music_dir`.
    ///
    /// The returned value is relative, contains no leading separator, and
    /// contains no parent-directory segment.
    ///
    /// # Errors
    ///
    /// Returns an error when `music_dir` or `absolute` is not absolute, when
    /// `absolute` is outside `music_dir`, or when the stored shape is invalid.
    pub fn from_absolute(music_dir: &Path, absolute: &Path) -> Result<Self> {
        anyhow::ensure!(
            music_dir.is_absolute(),
            "music_dir must be absolute: {}",
            music_dir.display()
        );
        anyhow::ensure!(
            absolute.is_absolute(),
            "local file path must be absolute: {}",
            absolute.display()
        );
        let relative = absolute.strip_prefix(music_dir).with_context(|| {
            format!(
                "local file path must be under music_dir {}: {}",
                music_dir.display(),
                absolute.display()
            )
        })?;
        let stored = relative
            .to_str()
            .with_context(|| format!("local file path must be UTF-8: {}", absolute.display()))?;
        Self::from_stored(stored)
    }

    /// Parses a stored relative local-file path.
    ///
    /// # Errors
    ///
    /// Returns an error when the stored value is empty, absolute, starts with a
    /// separator, or contains a parent-directory segment.
    pub(crate) fn from_stored(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_stored_path(&value)?;
        Ok(Self(value))
    }

    /// Resolves the stored path under `music_dir`.
    #[must_use]
    pub fn resolve(&self, music_dir: &Path) -> PathBuf {
        music_dir.join(&self.0)
    }

    /// Returns the database value for this path.
    #[must_use]
    pub(crate) fn as_stored(&self) -> &str {
        &self.0
    }

    /// Creates a stored path for unit-test fixtures.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn for_test(value: &str) -> Self {
        Self::from_stored(value).expect("test local path should be a valid stored relative path")
    }
}

fn validate_stored_path(value: &str) -> Result<()> {
    anyhow::ensure!(!value.is_empty(), "local file path cannot be empty");
    let path = Path::new(value);
    anyhow::ensure!(
        !path.is_absolute(),
        "local file path must be relative: {value}"
    );

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::ParentDir => {
                return Err(anyhow!(
                    "local file path cannot contain parent directory segments: {value}"
                ));
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("local file path has an invalid segment: {value}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_absolute_accepts_path_under_music_dir() -> Result<()> {
        let music_dir = Path::new("/music");
        let absolute = Path::new("/music/artists/artist/feed/track.mp3");

        let relative = LibraryRelativePath::from_absolute(music_dir, absolute)?;

        assert_eq!(relative.as_stored(), "artists/artist/feed/track.mp3");
        Ok(())
    }

    #[test]
    fn from_absolute_rejects_path_outside_music_dir() {
        let error =
            LibraryRelativePath::from_absolute(Path::new("/music"), Path::new("/other/track.mp3"))
                .expect_err("outside path should fail");

        assert!(error.to_string().contains("under music_dir"));
    }

    #[test]
    fn stored_path_rejects_leading_separator() {
        let error = LibraryRelativePath::from_stored("/artist/track.mp3")
            .expect_err("absolute stored path should fail");

        assert!(error.to_string().contains("relative"));
    }

    #[test]
    fn from_absolute_rejects_parent_directory_segment() {
        let error = LibraryRelativePath::from_absolute(
            Path::new("/music"),
            Path::new("/music/artist/../x.mp3"),
        )
        .expect_err("parent segment should fail");

        assert!(error.to_string().contains("parent directory"));
    }

    #[test]
    fn resolve_round_trips_with_from_absolute() -> Result<()> {
        let music_dir = Path::new("/music");
        let absolute = Path::new("/music/artists/artist/feed/track.mp3");
        let relative = LibraryRelativePath::from_absolute(music_dir, absolute)?;

        assert_eq!(relative.resolve(music_dir), absolute);
        Ok(())
    }
}
