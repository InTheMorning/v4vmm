//! Format-agnostic tag field identifiers.
//!
//! Today the pipeline speaks ID3v2.4 frame labels (e.g. `"TIT2"`, `"TXXX:V4V_PUBLISHER"`).
//! This module maps those labels to/from a `TagFieldId` that can be translated
//! into Vorbis Comments (FLAC/OGG) and iTunes-style MP4 atoms for Stage 5
//! writes.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TagFieldId {
    Title,
    Artist,
    AlbumArtist,
    Album,
    TrackNumber,
    TotalTracks,
    DiscNumber,
    Date,
    Composer,
    Genre,
    Publisher,
    Isrc,
    Comment,
    Lyrics,
    /// Free-form text with a descriptor (ID3 TXXX / Vorbis free key / MP4 ----)
    Custom(String),
    /// URL frame mapped by kind (ID3 WOAR, WOAF, WPUB ...)
    Url(UrlKind),
    /// Catch-all for ID3 text frames we don't special-case yet.
    Id3Text(String),
    /// Catch-all for ID3 URL frames we don't special-case yet.
    Id3Url(String),
    /// Catch-all for everything else (COMM:..., USLT:..., SYLT:..., UFID:..., etc.)
    Id3Raw(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UrlKind {
    OfficialArtist,
    OfficialAudio,
    Publisher,
    Commercial,
    Copyright,
    Source,
    RadioStation,
    Payment,
}

impl UrlKind {
    pub fn from_id3(id: &str) -> Option<Self> {
        match id {
            "WOAR" => Some(Self::OfficialArtist),
            "WOAF" => Some(Self::OfficialAudio),
            "WPUB" => Some(Self::Publisher),
            "WCOM" => Some(Self::Commercial),
            "WCOP" => Some(Self::Copyright),
            "WOAS" => Some(Self::Source),
            "WORS" => Some(Self::RadioStation),
            "WPAY" => Some(Self::Payment),
            _ => None,
        }
    }

    pub fn to_id3(self) -> &'static str {
        match self {
            Self::OfficialArtist => "WOAR",
            Self::OfficialAudio => "WOAF",
            Self::Publisher => "WPUB",
            Self::Commercial => "WCOM",
            Self::Copyright => "WCOP",
            Self::Source => "WOAS",
            Self::RadioStation => "WORS",
            Self::Payment => "WPAY",
        }
    }
}

impl TagFieldId {
    /// Map an ID3v2.4 frame label (possibly decorated with `:descriptor`) to a
    /// `TagFieldId`. Never returns `None`: unknown frames fall into one of the
    /// `Id3*` catch-all variants so the writer can still forward them.
    pub fn from_id3_label(label: &str) -> Self {
        let (head, desc) = label
            .split_once(':')
            .map(|(h, d)| (h, Some(d.to_string())))
            .unwrap_or((label, None));
        match head {
            "TIT2" => Self::Title,
            "TPE1" => Self::Artist,
            "TPE2" => Self::AlbumArtist,
            "TALB" => Self::Album,
            "TRCK" => Self::TrackNumber,
            "TPOS" => Self::DiscNumber,
            "TDRC" | "TYER" => Self::Date,
            "TCOM" => Self::Composer,
            "TCON" => Self::Genre,
            "TPUB" => Self::Publisher,
            "TSRC" => Self::Isrc,
            "COMM" => Self::Comment,
            "USLT" => Self::Lyrics,
            "TXXX" => desc.map(Self::Custom).unwrap_or_else(|| Self::Id3Text("TXXX".into())),
            other if other.starts_with('W') => {
                if let Some(kind) = UrlKind::from_id3(other) {
                    Self::Url(kind)
                } else {
                    Self::Id3Url(label.to_string())
                }
            }
            other if other.starts_with('T') => Self::Id3Text(label.to_string()),
            _ => Self::Id3Raw(label.to_string()),
        }
    }

    /// Vorbis Comment key (FLAC / OGG). Returns `None` for fields that have no
    /// standard Vorbis mapping (binary pictures, synchronised lyrics, ...).
    pub fn vorbis_key(&self) -> Option<String> {
        Some(match self {
            Self::Title => "TITLE".into(),
            Self::Artist => "ARTIST".into(),
            Self::AlbumArtist => "ALBUMARTIST".into(),
            Self::Album => "ALBUM".into(),
            Self::TrackNumber => "TRACKNUMBER".into(),
            Self::TotalTracks => "TRACKTOTAL".into(),
            Self::DiscNumber => "DISCNUMBER".into(),
            Self::Date => "DATE".into(),
            Self::Composer => "COMPOSER".into(),
            Self::Genre => "GENRE".into(),
            Self::Publisher => "ORGANIZATION".into(),
            Self::Isrc => "ISRC".into(),
            Self::Comment => "COMMENT".into(),
            Self::Lyrics => "LYRICS".into(),
            Self::Custom(desc) => desc.to_ascii_uppercase(),
            Self::Url(UrlKind::OfficialArtist) => "ARTISTWEBPAGE".into(),
            Self::Url(UrlKind::Publisher) => "PUBLISHERWEBPAGE".into(),
            Self::Url(UrlKind::OfficialAudio) => "AUDIOFILEWEBPAGE".into(),
            Self::Url(UrlKind::Source) => "AUDIOSOURCEWEBPAGE".into(),
            Self::Url(UrlKind::Commercial) => "COMMERCIALINFOURL".into(),
            Self::Url(UrlKind::Copyright) => "COPYRIGHTURL".into(),
            Self::Url(UrlKind::RadioStation) => "RADIOSTATIONWEBPAGE".into(),
            Self::Url(UrlKind::Payment) => "PAYMENTWEBPAGE".into(),
            Self::Id3Text(_) | Self::Id3Url(_) | Self::Id3Raw(_) => return None,
        })
    }

    /// iTunes-style atom name (MP4 / M4A). Returns `None` when there's no stock
    /// atom — the caller can fall back to a freeform `----:<ns>:<key>` atom.
    pub fn mp4_atom(&self) -> Option<&'static str> {
        Some(match self {
            Self::Title => "©nam",
            Self::Artist => "©ART",
            Self::AlbumArtist => "aART",
            Self::Album => "©alb",
            Self::TrackNumber | Self::TotalTracks => "trkn",
            Self::DiscNumber => "disk",
            Self::Date => "©day",
            Self::Composer => "©wrt",
            Self::Genre => "©gen",
            Self::Publisher => "©pub",
            Self::Comment => "©cmt",
            Self::Lyrics => "©lyr",
            _ => return None,
        })
    }

    /// Stable string identifier — used as a BTreeMap key when we want
    /// `TagFieldId`-keyed state without pulling the enum through the UI.
    pub fn stable_key(&self) -> String {
        match self {
            Self::Custom(desc) => format!("custom:{desc}"),
            Self::Url(kind) => format!("url:{}", kind.to_id3()),
            Self::Id3Text(label) | Self::Id3Url(label) | Self::Id3Raw(label) => label.clone(),
            other => format!("{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_id3_labels_map_to_named_variants() {
        assert_eq!(TagFieldId::from_id3_label("TIT2"), TagFieldId::Title);
        assert_eq!(TagFieldId::from_id3_label("TPE1"), TagFieldId::Artist);
        assert_eq!(TagFieldId::from_id3_label("TRCK"), TagFieldId::TrackNumber);
        assert_eq!(
            TagFieldId::from_id3_label("TXXX:V4V_PUBLISHER"),
            TagFieldId::Custom("V4V_PUBLISHER".into())
        );
        assert_eq!(
            TagFieldId::from_id3_label("WOAR"),
            TagFieldId::Url(UrlKind::OfficialArtist)
        );
    }

    #[test]
    fn unknown_frames_fall_back_to_raw_catchalls() {
        assert_eq!(
            TagFieldId::from_id3_label("TDOR"),
            TagFieldId::Id3Text("TDOR".into())
        );
        assert_eq!(
            TagFieldId::from_id3_label("UFID:http://musicbrainz.org"),
            TagFieldId::Id3Raw("UFID:http://musicbrainz.org".into())
        );
    }

    #[test]
    fn vorbis_and_mp4_keys_cover_common_fields() {
        assert_eq!(TagFieldId::Title.vorbis_key().as_deref(), Some("TITLE"));
        assert_eq!(
            TagFieldId::Custom("V4V_PUBLISHER".into()).vorbis_key().as_deref(),
            Some("V4V_PUBLISHER")
        );
        assert_eq!(TagFieldId::Title.mp4_atom(), Some("©nam"));
        assert_eq!(TagFieldId::AlbumArtist.mp4_atom(), Some("aART"));
        assert_eq!(TagFieldId::Id3Raw("PCST".into()).mp4_atom(), None);
    }
}
