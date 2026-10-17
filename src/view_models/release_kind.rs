//! The kind of a release (ADR 0084 Decision 5).
//!
//! A release is an Album, an EP or a Single. The stated value from the feed
//! wins when it names one of the three. Without a stated value, the track
//! count decides: one track gives a Single, and any other count gives an
//! Album. A count never gives an EP.

#![warn(clippy::pedantic)]

/// The kind of a release (ADR 0084 Decision 5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseKind {
    Album,
    Ep,
    Single,
}

impl ReleaseKind {
    /// Resolves the kind of a release (ADR 0084 Decision 5).
    ///
    /// `stated` is the free-text value that the feed states for its
    /// release kind, after trim, read without case. It wins when it names
    /// "album", "ep" or "single". Each other value, for example
    /// "unknown", counts as not stated.
    ///
    /// Without a stated value, `track_count` decides: one track gives a
    /// Single, and any other count, including zero, gives an Album. A
    /// track count never gives an EP.
    #[must_use]
    pub(crate) fn resolve(stated: Option<&str>, track_count: usize) -> Self {
        if let Some(kind) = stated.map(str::trim).and_then(Self::from_stated_value) {
            return kind;
        }
        if track_count == 1 {
            Self::Single
        } else {
            Self::Album
        }
    }

    /// Reads a stated release-kind value, read without case. Returns
    /// `None` for a value that names none of the three kinds.
    fn from_stated_value(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("album") {
            Some(Self::Album)
        } else if value.eq_ignore_ascii_case("ep") {
            Some(Self::Ep)
        } else if value.eq_ignore_ascii_case("single") {
            Some(Self::Single)
        } else {
            None
        }
    }

    /// The visible label: "Album", "EP" or "Single".
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Album => "Album",
            Self::Ep => "EP",
            Self::Single => "Single",
        }
    }

    /// The noun for an action label: "album", "EP" or "single".
    #[must_use]
    pub(crate) const fn noun(self) -> &'static str {
        match self {
            Self::Album => "album",
            Self::Ep => "EP",
            Self::Single => "single",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R7-01: a stated value wins over the track count, after trim and
    /// read without case. An unstated value falls back to the count rule,
    /// and a track count never gives an EP.
    #[test]
    fn adr_0084_release_kind_r7_01_resolve_follows_decision_5() {
        assert_eq!(ReleaseKind::resolve(Some("Album"), 1), ReleaseKind::Album);
        assert_eq!(ReleaseKind::resolve(Some("ep"), 1), ReleaseKind::Ep);
        assert_eq!(
            ReleaseKind::resolve(Some(" Single "), 5),
            ReleaseKind::Single
        );

        assert_eq!(
            ReleaseKind::resolve(Some("unknown"), 1),
            ReleaseKind::Single
        );
        assert_eq!(ReleaseKind::resolve(Some("unknown"), 5), ReleaseKind::Album);

        assert_eq!(ReleaseKind::resolve(None, 0), ReleaseKind::Album);
        assert_eq!(ReleaseKind::resolve(None, 12), ReleaseKind::Album);
    }

    /// R7-01: the visible label and the action noun of each kind.
    #[test]
    fn adr_0084_release_kind_label_and_noun() {
        assert_eq!(ReleaseKind::Album.label(), "Album");
        assert_eq!(ReleaseKind::Ep.label(), "EP");
        assert_eq!(ReleaseKind::Single.label(), "Single");

        assert_eq!(ReleaseKind::Album.noun(), "album");
        assert_eq!(ReleaseKind::Ep.noun(), "EP");
        assert_eq!(ReleaseKind::Single.noun(), "single");
    }
}
