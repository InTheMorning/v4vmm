use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Mp4,
    OggVorbis,
    OggOpus,
    Wav,
}

impl AudioFormat {
    pub fn from_declared_mime(mime: &str) -> Option<Self> {
        let norm = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        match norm.as_str() {
            "audio/mpeg" | "audio/mp3" | "audio/mpeg3" | "audio/x-mpeg-3" => Some(Self::Mp3),
            "audio/flac" | "audio/x-flac" => Some(Self::Flac),
            "audio/mp4" | "audio/x-m4a" | "audio/m4a" | "audio/aac" => Some(Self::Mp4),
            "audio/ogg" | "application/ogg" | "audio/vorbis" => Some(Self::OggVorbis),
            "audio/opus" => Some(Self::OggOpus),
            "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => Some(Self::Wav),
            _ => None,
        }
    }

    pub fn from_url_extension(url: &str) -> Option<Self> {
        let path = url.split('?').next().unwrap_or(url);
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        match ext.as_str() {
            "mp3" => Some(Self::Mp3),
            "flac" => Some(Self::Flac),
            "m4a" | "m4b" | "mp4" | "aac" => Some(Self::Mp4),
            "ogg" | "oga" => Some(Self::OggVorbis),
            "opus" => Some(Self::OggOpus),
            "wav" | "wave" => Some(Self::Wav),
            _ => None,
        }
    }

    pub fn detect_from_file(path: &Path) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("open {} for format detection", path.display()))?;
        let mut head = [0u8; 32];
        let n = file
            .read(&mut head)
            .with_context(|| format!("read {} for format detection", path.display()))?;
        Self::detect_from_bytes(&head[..n])
            .ok_or_else(|| anyhow::anyhow!("unknown audio format: {}", path.display()))
    }

    pub fn detect_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 4 && &bytes[..4] == b"fLaC" {
            return Some(Self::Flac);
        }
        if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
            return Some(Self::Wav);
        }
        if bytes.len() >= 4 && &bytes[..4] == b"OggS" {
            return Some(classify_ogg(bytes));
        }
        if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
            return Some(Self::Mp4);
        }
        if bytes.len() >= 3 && &bytes[..3] == b"ID3" {
            return Some(Self::Mp3);
        }
        if bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
            return Some(Self::Mp3);
        }
        None
    }

    pub fn canonical_extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Mp4 => "m4a",
            Self::OggVorbis => "ogg",
            Self::OggOpus => "opus",
            Self::Wav => "wav",
        }
    }

    pub fn canonical_mime(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Flac => "audio/flac",
            Self::Mp4 => "audio/mp4",
            Self::OggVorbis => "audio/ogg",
            Self::OggOpus => "audio/opus",
            Self::Wav => "audio/wav",
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::Mp3 => "MP3",
            Self::Flac => "FLAC",
            Self::Mp4 => "M4A",
            Self::OggVorbis => "OGG Vorbis",
            Self::OggOpus => "Opus",
            Self::Wav => "WAV",
        }
    }

    pub fn supports_tagging(self) -> bool {
        !matches!(self, Self::Wav)
    }
}

/// Returns true if the `flac` CLI is reachable on PATH. Probed once and cached.
pub fn flac_cli_available() -> bool {
    static PROBE: OnceLock<bool> = OnceLock::new();
    *PROBE.get_or_init(|| {
        Command::new("flac")
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    })
}

/// Re-encode a WAV file to FLAC in place (output alongside the input with the
/// `.flac` extension, removing the original WAV). Returns the new path on
/// success. Fails if the `flac` CLI is not installed or the encode fails.
pub fn transcode_wav_to_flac(wav_path: &Path) -> Result<std::path::PathBuf> {
    if !flac_cli_available() {
        anyhow::bail!("flac CLI is not installed");
    }
    let flac_path = wav_path.with_extension("flac");
    let status = Command::new("flac")
        .arg("--best")
        .arg("--silent")
        .arg("--totally-silent")
        .arg("--delete-input-file")
        .arg("-f")
        .arg("-o")
        .arg(&flac_path)
        .arg(wav_path)
        .status()
        .with_context(|| format!("invoke flac for {}", wav_path.display()))?;
    if !status.success() {
        anyhow::bail!(
            "flac failed (status {:?}) for {}",
            status.code(),
            wav_path.display()
        );
    }
    Ok(flac_path)
}

fn classify_ogg(bytes: &[u8]) -> AudioFormat {
    // Inspect the payload that follows the 27-byte Ogg page header + segment table.
    if bytes.len() < 28 {
        return AudioFormat::OggVorbis;
    }
    let seg_count = bytes[26] as usize;
    let payload_start = 27 + seg_count;
    if bytes.len() >= payload_start + 8 {
        let marker = &bytes[payload_start..payload_start + 8];
        if marker == b"OpusHead" {
            return AudioFormat::OggOpus;
        }
        if marker.len() >= 7 && &marker[1..7] == b"vorbis" {
            return AudioFormat::OggVorbis;
        }
    }
    AudioFormat::OggVorbis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_mime_covers_common_synonyms() {
        assert_eq!(
            AudioFormat::from_declared_mime("audio/mpeg"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("audio/MP3; codecs=mp3"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("audio/x-flac"),
            Some(AudioFormat::Flac)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("audio/mp4"),
            Some(AudioFormat::Mp4)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("audio/opus"),
            Some(AudioFormat::OggOpus)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("application/ogg"),
            Some(AudioFormat::OggVorbis)
        );
        assert_eq!(
            AudioFormat::from_declared_mime("audio/x-wav"),
            Some(AudioFormat::Wav)
        );
        assert_eq!(AudioFormat::from_declared_mime("text/html"), None);
    }

    #[test]
    fn url_extension_ignores_query() {
        assert_eq!(
            AudioFormat::from_url_extension("https://x/y.mp3?token=abc"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::from_url_extension("https://x/y.FLAC"),
            Some(AudioFormat::Flac)
        );
        assert_eq!(
            AudioFormat::from_url_extension("https://x/y.m4a"),
            Some(AudioFormat::Mp4)
        );
        assert_eq!(
            AudioFormat::from_url_extension("https://x/y.opus"),
            Some(AudioFormat::OggOpus)
        );
        assert_eq!(
            AudioFormat::from_url_extension("https://x/y.wav?x=1"),
            Some(AudioFormat::Wav)
        );
        assert_eq!(AudioFormat::from_url_extension("https://x/y"), None);
    }

    #[test]
    fn detect_recognises_magic_bytes() {
        assert_eq!(
            AudioFormat::detect_from_bytes(b"ID3\x04\x00\x00\x00\x00\x00\x00"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect_from_bytes(&[0xFF, 0xFB, 0x90, 0x00]),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect_from_bytes(b"fLaC\x00\x00\x00\x22"),
            Some(AudioFormat::Flac)
        );
        let mut wav = *b"RIFF\x00\x00\x00\x00WAVEfmt ";
        wav[4] = 0;
        assert_eq!(AudioFormat::detect_from_bytes(&wav), Some(AudioFormat::Wav));
        let mut mp4 = [0u8; 12];
        mp4[4..8].copy_from_slice(b"ftyp");
        mp4[8..12].copy_from_slice(b"M4A ");
        assert_eq!(AudioFormat::detect_from_bytes(&mp4), Some(AudioFormat::Mp4));
    }

    #[test]
    fn detect_distinguishes_ogg_vorbis_from_opus() {
        // Minimal Ogg page header (27 bytes) + one segment of length 8 + payload
        let mut page = vec![0u8; 27];
        page[0..4].copy_from_slice(b"OggS");
        page[26] = 1;
        page.push(8);
        page.extend_from_slice(b"OpusHead");
        assert_eq!(AudioFormat::detect_from_bytes(&page), Some(AudioFormat::OggOpus));

        let mut page = vec![0u8; 27];
        page[0..4].copy_from_slice(b"OggS");
        page[26] = 1;
        page.push(7);
        page.push(0x01);
        page.extend_from_slice(b"vorbis");
        assert_eq!(
            AudioFormat::detect_from_bytes(&page),
            Some(AudioFormat::OggVorbis)
        );
    }

    #[test]
    fn tagging_unsupported_for_wav() {
        assert!(!AudioFormat::Wav.supports_tagging());
        assert!(AudioFormat::Flac.supports_tagging());
        assert!(AudioFormat::Mp3.supports_tagging());
    }
}
