//! Music-root probes that never open existing audio (ADR 0066).

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{IssueSeverity, StartupIssue, StartupStage};

static PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const PROBE_BYTES: &[u8] = b"v4vmm music storage check\n";

pub(super) fn check_music(
    path: &Path,
    first_run: bool,
    prepare: bool,
) -> Result<Option<StartupIssue>, StartupIssue> {
    match fs::metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound && first_run => {
            if !prepare {
                let mut issue = StartupIssue::new(
                    StartupStage::MusicSetup,
                    Some(path),
                    "The new default music directory needs initial setup.",
                    "Choose Open app to create the first-run music directory.",
                );
                issue.severity = IssueSeverity::NeedsPreparation;
                return Ok(Some(issue));
            }
            fs::create_dir_all(path)
                .map_err(|e| StartupIssue::io(StartupStage::MusicSetup, path, &e))?;
        }
        Err(e) => return Err(StartupIssue::io(StartupStage::MusicInspect, path, &e)),
        Ok(metadata) if !metadata.is_dir() => {
            return Err(StartupIssue::new(
                StartupStage::MusicInspect,
                Some(path),
                "The music path points to a file, not a directory.",
                "Select a directory for music storage, then choose Check again.",
            ))
        }
        Ok(_) => {}
    }
    let mut entries =
        fs::read_dir(path).map_err(|e| StartupIssue::io(StartupStage::MusicList, path, &e))?;
    if let Some(entry) = entries.next() {
        entry.map_err(|e| StartupIssue::io(StartupStage::MusicList, path, &e))?;
    }
    probe_with(
        path,
        |file| {
            file.write_all(PROBE_BYTES)
                .map_err(|e| (StartupStage::MusicWriteProbe, e))?;
            file.sync_all()
                .map_err(|e| (StartupStage::MusicWriteProbe, e))?;
            file.seek(SeekFrom::Start(0))
                .map_err(|e| (StartupStage::MusicReadProbe, e))?;
            let mut actual = Vec::new();
            file.read_to_end(&mut actual)
                .map_err(|e| (StartupStage::MusicReadProbe, e))?;
            if actual != PROBE_BYTES {
                return Err((
                    StartupStage::MusicReadProbe,
                    io::Error::new(io::ErrorKind::InvalidData, "probe bytes differ"),
                ));
            }
            Ok(())
        },
        |probe| fs::remove_file(probe),
    )?;
    Ok(None)
}

fn probe_with(
    root: &Path,
    work: impl FnOnce(&mut fs::File) -> Result<(), (StartupStage, io::Error)>,
    cleanup: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), StartupIssue> {
    let path = root.join(format!(
        ".v4vmm-startup-probe-{}-{}",
        std::process::id(),
        PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| StartupIssue::io(StartupStage::MusicCreateProbe, &path, &e))?;
    let result = work(&mut file).map_err(|(stage, error)| StartupIssue::io(stage, &path, &error));
    drop(file);
    if let Err(error) = cleanup(&path) {
        let mut issue = StartupIssue::io(StartupStage::MusicRemoveProbe, &path, &error);
        issue.next_action = "Remove this remaining probe file after correcting permissions, then choose Check again.";
        if let Err(previous) = result {
            issue.cause = format!(
                "{} App also could not finish the probe: {}",
                issue.cause, previous.cause
            );
        }
        return Err(issue);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_music_root_is_not_recreated_for_existing_configuration() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("missing");
        assert_eq!(
            check_music(&root, false, true).unwrap_err().stage,
            StartupStage::MusicInspect
        );
        assert!(!root.exists());
        assert_eq!(
            check_music(&root, true, false).unwrap().unwrap().severity,
            IssueSeverity::NeedsPreparation
        );
        assert!(!root.exists());
        assert!(check_music(&root, true, true).unwrap().is_none());
        assert!(root.is_dir());
        assert_eq!(fs::read_dir(root).unwrap().count(), 0);
    }

    #[test]
    fn adr_0066_music_file_and_probe_failures_preserve_audio() {
        let temp = tempfile::tempdir().unwrap();
        let audio = temp.path().join("song.flac");
        fs::write(&audio, b"original music").unwrap();
        assert_eq!(
            check_music(&audio, false, false).unwrap_err().stage,
            StartupStage::MusicInspect
        );
        assert!(check_music(temp.path(), false, false).unwrap().is_none());
        for stage in [StartupStage::MusicWriteProbe, StartupStage::MusicReadProbe] {
            let issue = probe_with(
                temp.path(),
                |_| Err((stage, io::Error::other("injected"))),
                |p| fs::remove_file(p),
            )
            .unwrap_err();
            assert_eq!(issue.stage, stage);
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        }
        let issue = probe_with(
            temp.path(),
            |_| Ok(()),
            |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        )
        .unwrap_err();
        assert_eq!(issue.stage, StartupStage::MusicRemoveProbe);
        let residual = issue.resource.unwrap();
        assert!(residual.exists());
        fs::remove_file(residual).unwrap();
        assert_eq!(fs::read(audio).unwrap(), b"original music");
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_music_permissions_identify_list_and_write_failures() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("music");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
        let readonly = check_music(&root, false, false);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
        let unlistable = check_music(&root, false, false);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        // Privileged hosts may bypass mode bits; injected failures above cover
        // their error paths without depending on root's permission semantics.
        if let Err(issue) = readonly {
            assert_eq!(issue.stage, StartupStage::MusicCreateProbe);
        }
        if let Err(issue) = unlistable {
            assert_eq!(issue.stage, StartupStage::MusicList);
        }
    }
}
