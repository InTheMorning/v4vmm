//! The artist identity of a release (ADR 0084 Decision 7).
//!
//! A release that names its publisher feed has the publisher level. The
//! operator decided on 2026-10-08 that a release has the name level when
//! the agreed role of that link does not include artist. Each other
//! release has the name level, keyed by its artist text.

#![warn(clippy::pedantic)]

/// The `role_agreement` value that tells that both sides agree on the role
/// of a publisher link (ADR 0082 Decision 2).
const ROLE_AGREEMENT_BOTH: &str = "both";

/// The role item that keeps the publisher level (ADR 0084 Decision 7).
const ARTIST_ROLE: &str = "artist";

/// The artist identity of a release (ADR 0084 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ArtistIdentity {
    /// The publisher feed that the release names, by its feed GUID.
    Publisher(String),
    /// The artist text of a release that has no publisher level.
    Name(String),
}

impl ArtistIdentity {
    /// The publisher level when the release names a publisher feed, unless
    /// the agreed role of that link does not include artist. Otherwise the
    /// name level of `artist_text`.
    ///
    /// An empty or blank `publisher_feed_guid` counts as no publisher. The
    /// agreed role excludes artist when `role_agreement` is `both` and no
    /// item of `link_role`, split at `,` and trimmed, is `artist` without
    /// case. A null role with `both` also excludes artist. Each other
    /// agreement value keeps the publisher level.
    ///
    /// The name level keeps `artist_text` as it is. Two names that differ
    /// only in case are two identities.
    #[must_use]
    pub(crate) fn resolve(
        publisher_feed_guid: Option<&str>,
        link_role: Option<&str>,
        role_agreement: Option<&str>,
        artist_text: &str,
    ) -> Self {
        let publisher = publisher_feed_guid.filter(|guid| !guid.trim().is_empty());
        match publisher {
            Some(guid) if !agreed_role_excludes_artist(link_role, role_agreement) => {
                Self::Publisher(guid.to_owned())
            }
            _ => Self::Name(artist_text.to_owned()),
        }
    }
}

/// Tells if both sides agree on a role set that does not include artist
/// (ADR 0084 Decision 7, operator decision of 2026-10-08).
fn agreed_role_excludes_artist(link_role: Option<&str>, role_agreement: Option<&str>) -> bool {
    if role_agreement != Some(ROLE_AGREEMENT_BOTH) {
        return false;
    }
    !link_role.is_some_and(|role| {
        role.split(',')
            .map(str::trim)
            .any(|item| item.eq_ignore_ascii_case(ARTIST_ROLE))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R15-01: a publisher GUID keeps the publisher level unless both
    /// sides agree on a role set without artist. No GUID or a blank GUID
    /// gives the name level.
    #[test]
    fn adr_0084_singles_group_r15_01_resolve_follows_decision_7() {
        let publisher = ArtistIdentity::Publisher("pub-guid".to_owned());
        let name = ArtistIdentity::Name("Haleen".to_owned());
        let resolve =
            |guid, role, agreement| ArtistIdentity::resolve(guid, role, agreement, "Haleen");

        assert_eq!(resolve(Some("pub-guid"), Some("label"), None), publisher);
        assert_eq!(
            resolve(Some("pub-guid"), Some("label"), Some("one_side")),
            publisher
        );
        assert_eq!(
            resolve(Some("pub-guid"), Some("label"), Some("conflict")),
            publisher
        );
        assert_eq!(
            resolve(Some("pub-guid"), Some("artist, producer"), Some("both")),
            publisher
        );
        assert_eq!(
            resolve(Some("pub-guid"), Some("Producer , ARTIST"), Some("both")),
            publisher
        );
        assert_eq!(resolve(Some("pub-guid"), Some("label"), Some("both")), name);
        assert_eq!(resolve(Some("pub-guid"), None, Some("both")), name);
        assert_eq!(resolve(None, Some("artist"), Some("both")), name);
        assert_eq!(resolve(Some(""), None, None), name);
        assert_eq!(resolve(Some("   "), None, None), name);
    }

    /// R15-01: the name level keeps the artist text as it is, so a case
    /// difference gives two identities.
    #[test]
    fn adr_0084_singles_group_r15_01_name_level_keeps_case() {
        assert_ne!(
            ArtistIdentity::resolve(None, None, None, "Haleen"),
            ArtistIdentity::resolve(None, None, None, "haleen")
        );
        assert_ne!(
            ArtistIdentity::Publisher("Haleen".to_owned()),
            ArtistIdentity::Name("Haleen".to_owned())
        );
    }
}
