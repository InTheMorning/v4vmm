//! The name level of the artist page (ADR 0084 Decision 7, packet 017).
//!
//! A name level page lists the releases that state one artist text and name
//! no artist feed. It says that a name is not an identity, and it never
//! merges with a feed page. The page shows the Library releases of the name
//! and the `MusicIndex` releases of the name, each release one time.
//!
//! [`name_level_rows`] is the one row rule of this page and of the feed
//! page section "Matched by name". This module has no renderer type.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::view_models::artist_identity::{name_label, ArtistNameKey};
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::publisher_page::{
    PublisherPageAlbumVm, PublisherPageHeaderFact, PublisherPageSectionVm, PublisherPageVm,
    ReleaseRowOpenAction, TitleDisplay,
};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::workspace::ReleaseKey;

/// The facts of one Library release of a name (ADR 0084 Decision 7).
/// `LibraryViewModel::name_level_members` gives these values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameLevelLibraryRelease {
    /// The release key. `None` when the album has no feed GUID and no
    /// Library feed id.
    pub(crate) release_key: Option<ReleaseKey>,
    /// The feed GUID, when the release has one.
    pub(crate) feed_guid: Option<String>,
    /// The release title.
    pub(crate) title: String,
    /// The cover image URL.
    pub(crate) image_url: Option<String>,
    /// The raw artist text of the release: its `display_artist`.
    pub(crate) artist_text: String,
    /// The kind of the release (ADR 0084 Decision 5).
    pub(crate) kind: ReleaseKind,
    /// The newest `pub_date` of the tracks of the release.
    pub(crate) newest_date: Option<i64>,
    /// The state "in Library".
    pub(crate) in_library: bool,
}

/// The facts of one `MusicIndex` release of a name, from
/// `GET /v1/feeds/by-name?artist_key=<key>`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NameLevelIndexRelease {
    /// The feed GUID. The query drops a row with no feed GUID.
    pub(crate) feed_guid: String,
    /// The release title.
    pub(crate) title: Option<String>,
    /// The cover image URL.
    pub(crate) image_url: Option<String>,
    /// The raw artist text of the release.
    pub(crate) release_artist: Option<String>,
    /// The track count. A null count counts as 0.
    pub(crate) episode_count: Option<i32>,
    /// The date of the newest item.
    pub(crate) newest_item_at: Option<i64>,
}

/// The state of the `MusicIndex` part of a name level list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum NameLevelIndexState {
    /// The app sent no request, for example for the key of "Unknown
    /// Artist".
    #[default]
    NotRequested,
    /// A request is in flight.
    Loading,
    /// The releases came back. `more_available` is true when `MusicIndex`
    /// has more releases than the app requested.
    Loaded {
        releases: Vec<NameLevelIndexRelease>,
        more_available: bool,
    },
    /// The request failed. `detail` is the technical detail.
    Failed { detail: String },
}

impl NameLevelIndexState {
    /// One state for the requests of more than one key, in key order. A
    /// failed request wins, then a request in flight. Else the loaded
    /// releases of each key show together.
    #[must_use]
    pub(crate) fn combine<'a>(states: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut combined = Self::NotRequested;
        for state in states {
            combined = match (combined, state) {
                (failed @ Self::Failed { .. }, _) => failed,
                (_, Self::Failed { detail }) => Self::Failed {
                    detail: detail.clone(),
                },
                (Self::Loading, _) | (_, Self::Loading) => Self::Loading,
                (current, Self::NotRequested) => current,
                (Self::NotRequested, loaded @ Self::Loaded { .. }) => loaded.clone(),
                (
                    Self::Loaded {
                        mut releases,
                        more_available,
                    },
                    Self::Loaded {
                        releases: more,
                        more_available: more_flag,
                    },
                ) => {
                    releases.extend(more.iter().cloned());
                    Self::Loaded {
                        releases,
                        more_available: more_available || *more_flag,
                    }
                }
            };
        }
        combined
    }

    /// The status text of this state, when it has one (packet 017
    /// "The page view model").
    #[must_use]
    pub(crate) fn status(&self) -> Option<ReleaseListStatus> {
        match self {
            Self::NotRequested
            | Self::Loaded {
                more_available: false,
                ..
            } => None,
            Self::Loading => Some(ReleaseListStatus::Loading(NameLevelPageVm::LOADING_TEXT)),
            Self::Loaded {
                more_available: true,
                ..
            } => Some(ReleaseListStatus::MoreAvailable(
                NameLevelPageVm::MORE_RELEASES_TEXT,
            )),
            Self::Failed { detail } => Some(ReleaseListStatus::Failed {
                report: NameLevelPageVm::FAILURE_REPORT,
                detail: detail.clone(),
            }),
        }
    }
}

/// The status of the `MusicIndex` part of a release list, ready to view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseListStatus {
    /// The loading text.
    Loading(&'static str),
    /// The failure report, then the technical detail after it.
    Failed {
        report: &'static str,
        detail: String,
    },
    /// The text that tells that `MusicIndex` has more releases.
    MoreAvailable(&'static str),
}

/// One row of a name level list, with its kind for the split into
/// sections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameLevelRow {
    /// The kind of the release (ADR 0084 Decision 5).
    pub(crate) kind: ReleaseKind,
    /// The row, ready to view.
    pub(crate) row: PublisherPageAlbumVm,
}

/// The detail line of a row of the section "Matched by name" (ADR 0084
/// Decision 7, amendment of 2026-10-20). It tells that the relation is
/// inferred from the name.
#[must_use]
pub(crate) fn matched_by_name_line(artist_text: &str) -> String {
    format!("names \"{artist_text}\", no feed")
}

/// The rows of a name level list, newest first (packet 017 "The sections",
/// rules 1, 2, 5 and 6). The name level page and the feed page section
/// "Matched by name" both use this one rule.
///
/// - Each Library release gives one row, with "In Library".
/// - Each `MusicIndex` release gives one row, unless a Library row has its
///   feed GUID, or its feed GUID is in `other_identity_feed_guids`.
/// - With `matched_by_name`, each row has the detail line of
///   [`matched_by_name_line`].
#[must_use]
pub(crate) fn name_level_rows(
    library: &[NameLevelLibraryRelease],
    index: &NameLevelIndexState,
    other_identity_feed_guids: &BTreeSet<String>,
    matched_by_name: bool,
) -> Vec<NameLevelRow> {
    let mut dated: Vec<(Option<i64>, NameLevelRow)> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for release in library {
        if let Some(feed_guid) = &release.feed_guid {
            seen.insert(feed_guid);
        }
        let title = TitleDisplay::from_stated(
            Some(&release.title),
            release.feed_guid.as_deref().unwrap_or_default(),
        );
        let row = row_vm(
            release.feed_guid.clone(),
            title,
            release.image_url.clone(),
            release.in_library,
            matched_by_name.then(|| matched_by_name_line(&release.artist_text)),
            release.release_key.clone(),
        );
        dated.push((
            release.newest_date,
            NameLevelRow {
                kind: release.kind,
                row,
            },
        ));
    }
    if let NameLevelIndexState::Loaded { releases, .. } = index {
        for release in releases {
            if other_identity_feed_guids.contains(&release.feed_guid)
                || !seen.insert(&release.feed_guid)
            {
                continue;
            }
            let title = TitleDisplay::from_stated(release.title.as_deref(), &release.feed_guid);
            let artist_text = release.release_artist.as_deref().unwrap_or_default();
            let row = row_vm(
                Some(release.feed_guid.clone()),
                title,
                release.image_url.clone(),
                false,
                matched_by_name.then(|| matched_by_name_line(artist_text)),
                Some(ReleaseKey::from_index_feed_guid(release.feed_guid.clone())),
            );
            let track_count =
                usize::try_from(release.episode_count.unwrap_or(0).max(0)).unwrap_or(0);
            dated.push((
                release.newest_item_at,
                NameLevelRow {
                    kind: ReleaseKind::resolve(None, track_count),
                    row,
                },
            ));
        }
    }
    dated.sort_by(|(left_date, left), (right_date, right)| {
        right_date
            .is_some()
            .cmp(&left_date.is_some())
            .then_with(|| right_date.cmp(left_date))
            .then_with(|| {
                left.row
                    .title
                    .text()
                    .to_lowercase()
                    .cmp(&right.row.title.text().to_lowercase())
            })
    });
    dated.into_iter().map(|(_, row)| row).collect()
}

fn row_vm(
    feed_guid: Option<String>,
    title: TitleDisplay,
    image_url: Option<String>,
    in_library: bool,
    detail_line: Option<String>,
    key: Option<ReleaseKey>,
) -> PublisherPageAlbumVm {
    let open = key.map(|key| ReleaseRowOpenAction {
        key,
        a11y_label: format!("Open the release {}", title.text()),
    });
    PublisherPageAlbumVm {
        feed_guid,
        title,
        image_url,
        artist: None,
        not_listed: false,
        in_library,
        detail_line,
        open,
        selection: RowSelectionVm::none(),
    }
}

/// The notice of a name that now matches exactly one artist feed (packet
/// 017 decision D16).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchedFeedNoticeVm {
    /// The notice text.
    pub(crate) text: String,
    /// The feed GUID of the feed page that the open action opens.
    pub(crate) feed_guid: String,
    /// The visible label of the open action.
    pub(crate) action_label: String,
    /// The accessibility label of the open action.
    pub(crate) a11y_label: String,
}

/// The one artist feed that the name of a mounted page now matches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchedFeed {
    /// The feed GUID.
    pub(crate) feed_guid: String,
    /// The feed title.
    pub(crate) title: String,
}

/// The facts of one name level page (packet 017).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameLevelPageFacts {
    /// The key of the page.
    pub(crate) key: ArtistNameKey,
    /// The Library releases of the key, from `name_level_members` at each
    /// render.
    pub(crate) library: Vec<NameLevelLibraryRelease>,
    /// The `MusicIndex` part.
    pub(crate) index: NameLevelIndexState,
    /// Each feed GUID of a Library release whose identity is not this name.
    pub(crate) other_identity_feed_guids: BTreeSet<String>,
    /// The one artist feed that the key now matches, when a Library change
    /// made it match (decision D16).
    pub(crate) matched_feed: Option<MatchedFeed>,
}

/// The name level page, ready to view (ADR 0084 Decision 7, packet 017).
#[derive(Clone, Debug)]
pub(crate) struct NameLevelPageVm {
    facts: NameLevelPageFacts,
}

impl NameLevelPageVm {
    /// The frame title (packet 017 decision D5).
    pub(crate) const FRAME_TITLE: &'static str = "Artist name";
    /// The label of the confirmation level fact (ADR 0085 Decision 2).
    pub(crate) const LEVEL_LABEL: &'static str = PublisherPageVm::LEVEL_LABEL;
    /// The level word of the name level (ADR 0085 Decision 2).
    pub(crate) const LEVEL_TEXT: &'static str = "Name only";
    /// The label of the note fact.
    pub(crate) const NOTE_LABEL: &'static str = "About this page";
    /// The note that a name is not an identity (packet 017 decision D4).
    pub(crate) const NOTE_TEXT: &'static str =
        "A name is not an identity. These releases state this artist text and name no artist feed.";
    /// The label of the Library count fact.
    pub(crate) const IN_LIBRARY_LABEL: &'static str = "In Library";
    /// The label of the count of all rows.
    pub(crate) const RELEASES_LABEL: &'static str = "Releases";
    /// The loading text of the `MusicIndex` part.
    pub(crate) const LOADING_TEXT: &'static str =
        "Loading the releases of this name from MusicIndex.";
    /// The report of a failed `MusicIndex` request.
    pub(crate) const FAILURE_REPORT: &'static str = "The app could not load the releases of this name from MusicIndex. The releases in the Library stay on this page.";
    /// The text that tells that `MusicIndex` has more releases (decision
    /// D10).
    pub(crate) const MORE_RELEASES_TEXT: &'static str =
        "MusicIndex has more releases for this name. This page shows the first 1,000.";
    /// The label of the open action of the notice of decision D16.
    pub(crate) const OPEN_ARTIST_PAGE_LABEL: &'static str = "Open the artist page";

    #[must_use]
    pub(crate) const fn new(facts: NameLevelPageFacts) -> Self {
        Self { facts }
    }

    /// The page title of a name, for a search row before a page loads.
    #[must_use]
    pub(crate) fn title_text_for_name(name: &str) -> String {
        format!("Releases that name {name}")
    }

    /// The label of the name: the raw text that the most rows state, over
    /// the Library rows and the `MusicIndex` rows. With no row, it is the
    /// label of the entry.
    #[must_use]
    pub(crate) fn label(&self) -> String {
        let index_texts: Vec<&str> = match &self.facts.index {
            NameLevelIndexState::Loaded { releases, .. } => releases
                .iter()
                .filter_map(|release| release.release_artist.as_deref())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .collect(),
            NameLevelIndexState::NotRequested
            | NameLevelIndexState::Loading
            | NameLevelIndexState::Failed { .. } => Vec::new(),
        };
        name_label(
            self.facts
                .library
                .iter()
                .map(|release| release.artist_text.as_str())
                .chain(index_texts),
        )
        .unwrap_or_else(|| self.facts.key.label.clone())
    }

    /// The page title: "Releases that name <label>".
    #[must_use]
    pub(crate) fn title_text(&self) -> String {
        Self::title_text_for_name(&self.label())
    }

    fn rows(&self) -> Vec<NameLevelRow> {
        if self.facts.matched_feed.is_some() {
            return Vec::new();
        }
        name_level_rows(
            &self.facts.library,
            &self.facts.index,
            &self.facts.other_identity_feed_guids,
            false,
        )
    }

    /// The header facts. "Releases" shows only when the `MusicIndex` part
    /// loaded. "In Library" and "Releases" are two facts, never one ratio.
    #[must_use]
    pub(crate) fn header_facts(&self) -> Vec<PublisherPageHeaderFact> {
        let mut facts = vec![
            PublisherPageHeaderFact {
                label: Self::LEVEL_LABEL,
                value: Self::LEVEL_TEXT.to_owned(),
            },
            PublisherPageHeaderFact {
                label: Self::NOTE_LABEL,
                value: Self::NOTE_TEXT.to_owned(),
            },
            PublisherPageHeaderFact {
                label: Self::IN_LIBRARY_LABEL,
                value: self.facts.library.len().to_string(),
            },
        ];
        if matches!(self.facts.index, NameLevelIndexState::Loaded { .. }) {
            facts.push(PublisherPageHeaderFact {
                label: Self::RELEASES_LABEL,
                value: self.rows().len().to_string(),
            });
        }
        facts
    }

    /// The sections: "Albums and EPs", then "Singles". A section with no
    /// row is absent. A page whose name now matches one feed has no
    /// section (decision D16).
    #[must_use]
    pub(crate) fn sections(&self) -> Vec<PublisherPageSectionVm> {
        let mut albums_and_eps = Vec::new();
        let mut singles = Vec::new();
        for row in self.rows() {
            if row.kind.is_single() {
                singles.push(row.row);
            } else {
                albums_and_eps.push(row.row);
            }
        }
        let mut sections = Vec::new();
        for (heading, rows) in [
            (PublisherPageVm::ALBUMS_AND_EPS_HEADING, albums_and_eps),
            (PublisherPageVm::SINGLES_HEADING, singles),
        ] {
            if !rows.is_empty() {
                sections.push(PublisherPageSectionVm {
                    heading: heading.to_owned(),
                    rows,
                    note: None,
                    status: None,
                });
            }
        }
        sections
    }

    /// The status of the `MusicIndex` part: the loading text, the failure
    /// report, or the "more releases" text.
    #[must_use]
    pub(crate) fn index_status(&self) -> Option<ReleaseListStatus> {
        if self.facts.matched_feed.is_some() {
            return None;
        }
        self.facts.index.status()
    }

    /// The header art (ADR 0084 Decision 12): the logo mosaic of the
    /// newest four covers, one cover as a square, or the entity color. The
    /// name level gives no provided image.
    #[must_use]
    pub(crate) fn collection_art(&self) -> CollectionArtVm {
        let rows = self.rows();
        let covers: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.row.image_url.as_deref())
            .collect();
        CollectionArtVm::resolve(None, covers, CollectionKind::Artist)
    }

    /// Each distinct cover URL of the rows, for the app layer to resolve.
    #[must_use]
    pub(crate) fn album_image_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .rows()
            .into_iter()
            .filter_map(|row| row.row.image_url)
            .collect();
        urls.sort_unstable();
        urls.dedup();
        urls
    }

    /// The text of a page with no release. It shows only when each section
    /// is empty and the `MusicIndex` part loaded.
    #[must_use]
    pub(crate) fn empty_message(&self) -> Option<String> {
        let loaded = matches!(self.facts.index, NameLevelIndexState::Loaded { .. });
        (loaded && self.facts.matched_feed.is_none() && self.rows().is_empty()).then(|| {
            format!(
                "No release in the Library or in MusicIndex names {} as its artist and names no artist feed.",
                self.label()
            )
        })
    }

    /// The notice of a name that now matches exactly one artist feed
    /// (decision D16), with the open action of that feed page.
    #[must_use]
    pub(crate) fn matched_feed_notice(&self) -> Option<MatchedFeedNoticeVm> {
        let feed = self.facts.matched_feed.as_ref()?;
        Some(MatchedFeedNoticeVm {
            text: format!(
                "The releases of this name now show on the artist page {}.",
                feed.title
            ),
            feed_guid: feed.feed_guid.clone(),
            action_label: Self::OPEN_ARTIST_PAGE_LABEL.to_owned(),
            a11y_label: format!("Open the artist page {}", feed.title),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library_release(
        guid: &str,
        title: &str,
        kind: ReleaseKind,
        date: Option<i64>,
    ) -> NameLevelLibraryRelease {
        NameLevelLibraryRelease {
            release_key: Some(ReleaseKey::from_index_feed_guid(guid)),
            feed_guid: Some(guid.to_owned()),
            title: title.to_owned(),
            image_url: Some(format!("https://example.test/{guid}.jpg")),
            artist_text: "Haleen".to_owned(),
            kind,
            newest_date: date,
            in_library: true,
        }
    }

    fn index_release(
        guid: &str,
        title: &str,
        count: Option<i32>,
        date: Option<i64>,
    ) -> NameLevelIndexRelease {
        NameLevelIndexRelease {
            feed_guid: guid.to_owned(),
            title: Some(title.to_owned()),
            image_url: None,
            release_artist: Some("Haleen".to_owned()),
            episode_count: count,
            newest_item_at: date,
        }
    }

    fn facts(
        library: Vec<NameLevelLibraryRelease>,
        index: NameLevelIndexState,
    ) -> NameLevelPageFacts {
        NameLevelPageFacts {
            key: ArtistNameKey::new("haleen", "Haleen"),
            library,
            index,
            other_identity_feed_guids: BTreeSet::new(),
            matched_feed: None,
        }
    }

    fn titles(section: &PublisherPageSectionVm) -> Vec<&str> {
        section.rows.iter().map(|row| row.title.text()).collect()
    }

    /// R17-06: "Albums and EPs" before "Singles", the kind of each row, an
    /// absent empty section, and newest first with no date last.
    #[test]
    fn adr_0084_name_level_r17_06_sections() {
        let vm = NameLevelPageVm::new(facts(
            vec![
                library_release(
                    "lib-single",
                    "Library single",
                    ReleaseKind::Single,
                    Some(10),
                ),
                library_release("lib-album", "Library album", ReleaseKind::Album, None),
            ],
            NameLevelIndexState::Loaded {
                releases: vec![
                    index_release("index-single", "Index single", Some(1), Some(30)),
                    index_release("index-null", "Index null count", None, Some(20)),
                ],
                more_available: false,
            },
        ));
        let sections = vm.sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "Albums and EPs");
        assert_eq!(titles(&sections[0]), ["Index null count", "Library album"]);
        assert_eq!(sections[1].heading, "Singles");
        assert_eq!(titles(&sections[1]), ["Index single", "Library single"]);

        let singles_only = NameLevelPageVm::new(facts(
            vec![library_release("a", "A", ReleaseKind::Single, Some(1))],
            NameLevelIndexState::NotRequested,
        ));
        let sections = singles_only.sections();
        assert_eq!(sections.len(), 1, "an empty section is absent");
        assert_eq!(sections[0].heading, "Singles");
    }

    /// R17-07: a `MusicIndex` row with the feed GUID of a Library row shows
    /// one time, with "In Library". A row of another identity is absent.
    /// A `MusicIndex`-only row has `in_library` false.
    #[test]
    fn adr_0084_name_level_r17_07_one_row_for_each_release() {
        let mut page_facts = facts(
            vec![library_release(
                "shared",
                "Shared",
                ReleaseKind::Album,
                Some(5),
            )],
            NameLevelIndexState::Loaded {
                releases: vec![
                    index_release("shared", "Shared", Some(9), Some(5)),
                    index_release("other", "Other identity", Some(9), Some(4)),
                    index_release("index-only", "Index only", Some(9), Some(3)),
                ],
                more_available: false,
            },
        );
        page_facts
            .other_identity_feed_guids
            .insert("other".to_owned());
        let vm = NameLevelPageVm::new(page_facts);
        let rows: Vec<PublisherPageAlbumVm> = vm
            .sections()
            .into_iter()
            .flat_map(|section| section.rows)
            .collect();
        let shown: Vec<(&str, bool)> = rows
            .iter()
            .map(|row| (row.title.text(), row.in_library))
            .collect();
        assert_eq!(shown, [("Shared", true), ("Index only", false)]);
        assert!(rows.iter().all(|row| row.detail_line.is_none()));
        assert_eq!(
            rows[1].open.as_ref().map(|open| open.a11y_label.as_str()),
            Some("Open the release Index only")
        );
    }

    /// R17-08: the title, the header facts and the frame title.
    #[test]
    fn adr_0084_name_level_r17_08_titles_and_facts() {
        let library = vec![library_release("a", "A", ReleaseKind::Album, Some(1))];
        let loading = NameLevelPageVm::new(facts(library.clone(), NameLevelIndexState::Loading));
        assert_eq!(loading.title_text(), "Releases that name Haleen");
        assert_eq!(NameLevelPageVm::FRAME_TITLE, "Artist name");
        let pairs: Vec<(&str, String)> = loading
            .header_facts()
            .into_iter()
            .map(|fact| (fact.label, fact.value))
            .collect();
        assert_eq!(
            pairs,
            [
                ("Confirmation", "Name only".to_owned()),
                ("About this page", NameLevelPageVm::NOTE_TEXT.to_owned()),
                ("In Library", "1".to_owned()),
            ],
            "\"Releases\" shows only when MusicIndex loaded"
        );

        let loaded = NameLevelPageVm::new(facts(
            library,
            NameLevelIndexState::Loaded {
                releases: vec![index_release("b", "B", Some(1), Some(2))],
                more_available: false,
            },
        ));
        let releases = loaded
            .header_facts()
            .into_iter()
            .find(|fact| fact.label == "Releases")
            .map(|fact| fact.value);
        assert_eq!(releases.as_deref(), Some("2"));

        let empty = NameLevelPageVm::new(facts(Vec::new(), NameLevelIndexState::NotRequested));
        assert_eq!(empty.title_text(), "Releases that name Haleen");
    }

    /// R17-09: the loading text, the failure report before the detail
    /// with the Library rows kept, the "more releases" text and the empty
    /// text.
    #[test]
    fn adr_0084_name_level_r17_09_index_states() {
        let library = vec![library_release("a", "A", ReleaseKind::Album, Some(1))];
        let loading = NameLevelPageVm::new(facts(library.clone(), NameLevelIndexState::Loading));
        assert_eq!(
            loading.index_status(),
            Some(ReleaseListStatus::Loading(NameLevelPageVm::LOADING_TEXT))
        );

        let failed = NameLevelPageVm::new(facts(
            library.clone(),
            NameLevelIndexState::Failed {
                detail: "connection refused".to_owned(),
            },
        ));
        assert_eq!(
            failed.index_status(),
            Some(ReleaseListStatus::Failed {
                report: NameLevelPageVm::FAILURE_REPORT,
                detail: "connection refused".to_owned(),
            })
        );
        assert_eq!(failed.sections().len(), 1, "the Library rows stay");
        assert_eq!(failed.empty_message(), None);

        let more = NameLevelPageVm::new(facts(
            library,
            NameLevelIndexState::Loaded {
                releases: Vec::new(),
                more_available: true,
            },
        ));
        assert_eq!(
            more.index_status(),
            Some(ReleaseListStatus::MoreAvailable(
                NameLevelPageVm::MORE_RELEASES_TEXT
            ))
        );

        let empty = NameLevelPageVm::new(facts(
            Vec::new(),
            NameLevelIndexState::Loaded {
                releases: Vec::new(),
                more_available: false,
            },
        ));
        assert_eq!(
            empty.empty_message().as_deref(),
            Some("No release in the Library or in MusicIndex names Haleen as its artist and names no artist feed.")
        );
    }

    /// R17-17: the mosaic of the newest four covers, one cover as a square,
    /// and the entity color with no cover. No provided image.
    #[test]
    fn adr_0084_name_level_r17_17_collection_art() {
        let library: Vec<NameLevelLibraryRelease> = (0..5)
            .map(|day| library_release(&format!("g{day}"), "T", ReleaseKind::Album, Some(day)))
            .collect();
        let vm = NameLevelPageVm::new(facts(library, NameLevelIndexState::NotRequested));
        assert_eq!(
            vm.collection_art(),
            CollectionArtVm::resolve(
                None,
                ["g4", "g3", "g2", "g1"].map(|guid| format!("https://example.test/{guid}.jpg")),
                CollectionKind::Artist,
            )
        );
        assert!(matches!(vm.collection_art(), CollectionArtVm::Mosaic(_)));

        let one = NameLevelPageVm::new(facts(
            vec![library_release("g", "T", ReleaseKind::Album, None)],
            NameLevelIndexState::NotRequested,
        ));
        assert_eq!(
            one.collection_art(),
            CollectionArtVm::SingleCover("https://example.test/g.jpg".to_owned())
        );

        let none = NameLevelPageVm::new(facts(Vec::new(), NameLevelIndexState::NotRequested));
        assert_eq!(
            none.collection_art(),
            CollectionArtVm::EntityColor(CollectionKind::Artist)
        );
    }

    /// R17-30: a key that now matches one feed shows the notice with the
    /// feed title and its open action, and no section.
    #[test]
    fn adr_0084_name_level_r17_30_matched_feed_notice() {
        let mut page_facts = facts(
            vec![library_release("a", "A", ReleaseKind::Album, Some(1))],
            NameLevelIndexState::Loading,
        );
        page_facts.matched_feed = Some(MatchedFeed {
            feed_guid: "haleen-guid".to_owned(),
            title: "Haleen".to_owned(),
        });
        let vm = NameLevelPageVm::new(page_facts);
        assert!(vm.sections().is_empty());
        assert_eq!(vm.index_status(), None);
        let notice = vm.matched_feed_notice().expect("a notice");
        assert_eq!(
            notice.text,
            "The releases of this name now show on the artist page Haleen."
        );
        assert_eq!(notice.feed_guid, "haleen-guid");
        assert_eq!(notice.a11y_label, "Open the artist page Haleen");
    }

    /// The state of more than one key: a failure wins, then loading. Else
    /// the releases of each key show together.
    #[test]
    fn adr_0084_name_level_combined_index_state() {
        let loaded = |guid: &str, more: bool| NameLevelIndexState::Loaded {
            releases: vec![index_release(guid, guid, Some(1), None)],
            more_available: more,
        };
        let failed = NameLevelIndexState::Failed {
            detail: "error".to_owned(),
        };
        assert_eq!(
            NameLevelIndexState::combine([&loaded("a", false), &failed]),
            failed
        );
        assert_eq!(
            NameLevelIndexState::combine([&NameLevelIndexState::Loading, &loaded("a", false)]),
            NameLevelIndexState::Loading
        );
        let NameLevelIndexState::Loaded {
            releases,
            more_available,
        } = NameLevelIndexState::combine([&loaded("a", false), &loaded("b", true)])
        else {
            panic!("two loaded states give one loaded state");
        };
        assert_eq!(releases.len(), 2);
        assert!(more_available);
        assert_eq!(
            NameLevelIndexState::combine(std::iter::empty()),
            NameLevelIndexState::NotRequested
        );
    }
}
