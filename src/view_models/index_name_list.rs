//! The scope control and the `MusicIndex` name list pages of the Releases
//! and Artists lists (ADR 0084 Decision 8, packet 016).
//!
//! A list in the scope "Library" shows the Library items. A list in the
//! scope "Everything" adds the `MusicIndex` items of the name lists of
//! Stophammer ADR 0070, in one name order. This module gives the scope
//! control, the sort key, the page state of one name list, the watermark
//! and the status row. It has no renderer type.

#![warn(clippy::pedantic)]

use crate::view_models::workspace::MusicDestination;

/// The scope of the Releases list or the Artists list (ADR 0084 Decision
/// 8). Each app start uses `Library` (packet 016 decision D3).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MusicListScope {
    /// The Library items only.
    #[default]
    Library,
    /// The Library items and the `MusicIndex` items, in one name order.
    Everything,
}

/// One option of the scope control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MusicScopeOptionDisplay {
    /// Stable element id.
    pub(crate) id: &'static str,
    /// The scope that this option selects.
    pub(crate) scope: MusicListScope,
    /// The visible label.
    pub(crate) label: &'static str,
    /// The accessibility label.
    pub(crate) a11y_label: &'static str,
}

/// The scope control of the Releases list or the Artists list (ADR 0084
/// Decision 8, packet 016 decisions D1 and D2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MusicScopeControlDisplay {
    /// Stable element id of the control.
    pub(crate) control_id: &'static str,
    /// The selected scope.
    pub(crate) selected: MusicListScope,
    /// The two options, "Library" first.
    pub(crate) options: [MusicScopeOptionDisplay; 2],
}

impl MusicScopeControlDisplay {
    /// The control of `destination` with `selected` as its value. Only
    /// Releases and Artists have a scope. Each other destination gives
    /// `None`.
    #[must_use]
    pub(crate) const fn for_destination(
        destination: MusicDestination,
        selected: MusicListScope,
    ) -> Option<Self> {
        let (control_id, ids, a11y_labels) = match destination {
            MusicDestination::Releases => (
                "music-scope-releases",
                [
                    "music-scope-releases-library",
                    "music-scope-releases-everything",
                ],
                [
                    "Show Library releases only",
                    "Show Library and MusicIndex releases",
                ],
            ),
            MusicDestination::Artists => (
                "music-scope-artists",
                [
                    "music-scope-artists-library",
                    "music-scope-artists-everything",
                ],
                [
                    "Show Library artists only",
                    "Show Library and MusicIndex artists",
                ],
            ),
            MusicDestination::Discover
            | MusicDestination::Tracks
            | MusicDestination::NeedsAttention
            | MusicDestination::Playlists => return None,
        };
        Some(Self {
            control_id,
            selected,
            options: [
                MusicScopeOptionDisplay {
                    id: ids[0],
                    scope: MusicListScope::Library,
                    label: "Library",
                    a11y_label: a11y_labels[0],
                },
                MusicScopeOptionDisplay {
                    id: ids[1],
                    scope: MusicListScope::Everything,
                    label: "Everything",
                    a11y_label: a11y_labels[1],
                },
            ],
        })
    }
}

/// The sort key of a name (Stophammer ADR 0070 §1, ADR 0084 Decision 7).
/// It removes the white space at the two ends, makes each internal run of
/// white space one space, and makes each character Unicode lowercase. It
/// keeps an article such as "The" and each punctuation mark. Rust `String`
/// order is code point order, as the Stophammer order is.
#[must_use]
pub(crate) fn name_sort_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The watermark of packet 016 decision D7. While `MusicIndex` has more
/// pages, a row shows only when its order position is at or before the
/// last loaded `MusicIndex` row. Thus a row never moves after it shows.
/// When `MusicIndex` has no more pages, each row shows.
#[must_use]
pub(crate) fn watermark_shows<K: Ord>(key: &K, last_loaded: Option<&K>, has_more: bool) -> bool {
    !has_more || last_loaded.is_some_and(|last| key <= last)
}

/// One page of `MusicIndex` name list rows and its cursor.
#[derive(Clone, Debug)]
pub(crate) struct IndexNameListBatch<Row> {
    /// The rows of this page, in sort key order.
    pub(crate) rows: Vec<Row>,
    /// The cursor of the next page, when there is one.
    pub(crate) cursor: Option<String>,
    /// Whether `MusicIndex` has more pages.
    pub(crate) has_more: bool,
}

/// The request that [`IndexNameListPage::begin_load`] asks for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexNameListLoadIntent {
    /// The cursor to send. `None` asks for page 1.
    pub(crate) cursor: Option<String>,
    /// The request generation. A result of an older generation changes
    /// nothing (packet 016 decision D10).
    pub(crate) generation: u64,
}

/// The load state of one name list.
#[derive(Clone, Debug, Eq, PartialEq)]
enum IndexNameListLoadState {
    /// No request was sent.
    NotStarted,
    /// A request is in flight. `append` is false for page 1.
    Loading { append: bool },
    /// The last request gave its page.
    Loaded,
    /// The last request failed. `append` is false for page 1. `request`
    /// names the failed request when it is not the name list request, for
    /// example a feed request of the page (ADR 0084 packet 026).
    Failed {
        append: bool,
        detail: String,
        request: Option<String>,
    },
}

/// The loaded pages of one `MusicIndex` name list (ADR 0084 packet 016).
/// The pages stay until the app stops. No page expires. "Try again" after
/// an error on page 1 is the only new load of page 1 (decision D9).
#[derive(Clone, Debug)]
pub(crate) struct IndexNameListPage<Row> {
    rows: Vec<Row>,
    cursor: Option<String>,
    has_more: bool,
    state: IndexNameListLoadState,
    generation: u64,
    /// The rows of a page whose name list request had an answer while
    /// another request of the page failed (ADR 0084 packet 026 decision
    /// D10). They show only after "Try again" gets each answer.
    pending: Option<IndexNameListBatch<Row>>,
}

impl<Row> Default for IndexNameListPage<Row> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            cursor: None,
            has_more: false,
            state: IndexNameListLoadState::NotStarted,
            generation: 0,
            pending: None,
        }
    }
}

impl<Row> IndexNameListPage<Row> {
    /// The loaded rows, in sort key order.
    #[must_use]
    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Whether `MusicIndex` has more pages.
    #[must_use]
    pub(crate) const fn has_more(&self) -> bool {
        self.has_more
    }

    /// Whether no request was sent.
    #[must_use]
    pub(crate) const fn is_not_started(&self) -> bool {
        matches!(self.state, IndexNameListLoadState::NotStarted)
    }

    /// Whether the request of page 1 is in flight. The list then shows no
    /// item row (decision D8).
    #[must_use]
    pub(crate) const fn first_page_in_flight(&self) -> bool {
        matches!(
            self.state,
            IndexNameListLoadState::Loading { append: false }
        )
    }

    /// Begins a load of page 1 (`append` false) or of the next page
    /// (`append` true). A load of page 1 removes the loaded rows and adds 1
    /// to the generation. An append gives `None` while a request is in
    /// flight, or when `MusicIndex` has no more pages.
    #[must_use]
    pub(crate) fn begin_load(&mut self, append: bool) -> Option<IndexNameListLoadIntent> {
        if append {
            if matches!(self.state, IndexNameListLoadState::Loading { .. }) || !self.has_more {
                return None;
            }
            self.pending = None;
            self.state = IndexNameListLoadState::Loading { append: true };
            return Some(IndexNameListLoadIntent {
                cursor: self.cursor.clone(),
                generation: self.generation,
            });
        }
        self.generation += 1;
        self.pending = None;
        self.rows.clear();
        self.cursor = None;
        self.has_more = false;
        self.state = IndexNameListLoadState::Loading { append: false };
        Some(IndexNameListLoadIntent {
            cursor: None,
            generation: self.generation,
        })
    }

    /// Begins the "Try again" of a page whose rows wait (ADR 0084 packet 026
    /// decision D10). Gives the waiting rows and the generation, and starts the load state again. The caller sends only the failed
    /// requests of the page. Gives `None` when no rows wait.
    #[must_use]
    pub(crate) fn begin_pending_retry(&mut self) -> Option<(IndexNameListBatch<Row>, u64)> {
        let IndexNameListLoadState::Failed { append, .. } = self.state else {
            return None;
        };
        let pending = self.pending.take()?;
        self.state = IndexNameListLoadState::Loading { append };
        Some((pending, self.generation))
    }

    /// Begins the load that "Try again" asks for: page 1 after an error on
    /// page 1, or the same next page after an error on a later page. Gives
    /// `None` when the last request did not fail.
    #[must_use]
    pub(crate) fn begin_retry(&mut self) -> Option<IndexNameListLoadIntent> {
        let IndexNameListLoadState::Failed { append, .. } = self.state else {
            return None;
        };
        self.begin_load(append)
    }

    /// True when `generation` is the generation of the request in flight.
    #[must_use]
    pub(crate) const fn is_current(&self, generation: u64) -> bool {
        generation == self.generation
    }

    /// Adds the rows of `batch`. Gives `false` and changes nothing when
    /// `generation` is old.
    pub(crate) fn finish_load(&mut self, batch: IndexNameListBatch<Row>, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.rows.extend(batch.rows);
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;
        self.state = IndexNameListLoadState::Loaded;
        true
    }

    /// Records a failed request. The loaded rows stay. Gives `false` and
    /// changes nothing when `generation` is old.
    pub(crate) fn fail_load(&mut self, detail: impl Into<String>, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        let append = matches!(self.state, IndexNameListLoadState::Loading { append: true });
        self.state = IndexNameListLoadState::Failed {
            append,
            detail: detail.into(),
            request: None,
        };
        true
    }

    /// Records a page whose name list request had an answer while another
    /// request of the page failed (ADR 0084 packet 026 decision D10). The
    /// rows of `batch` wait, and the error row names `request`. Gives
    /// `false` and changes nothing when `generation` is old.
    pub(crate) fn fail_pending(
        &mut self,
        batch: IndexNameListBatch<Row>,
        request: String,
        detail: impl Into<String>,
        generation: u64,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        let append = matches!(self.state, IndexNameListLoadState::Loading { append: true });
        self.state = IndexNameListLoadState::Failed {
            append,
            detail: detail.into(),
            request: Some(request),
        };
        self.pending = Some(batch);
        true
    }

    /// The status row at the end of the list, when it has one (decision
    /// D8): the loading row of page 1, the error row with "Try again", or
    /// "Load more".
    #[must_use]
    pub(crate) fn status_row(&self, list: IndexNameList) -> Option<IndexListStatusRowVm> {
        match &self.state {
            IndexNameListLoadState::NotStarted => None,
            IndexNameListLoadState::Loading { append: false } => {
                Some(IndexListStatusRowVm::Loading {
                    element_id: list.element_id("loading"),
                    label: list.loading_label(),
                })
            }
            IndexNameListLoadState::Loading { append: true } => Some(list.load_more_row(false)),
            IndexNameListLoadState::Loaded => self.has_more.then(|| list.load_more_row(true)),
            IndexNameListLoadState::Failed {
                append,
                detail,
                request,
            } => Some(IndexListStatusRowVm::Failed {
                element_id: list.element_id("error"),
                first_page: !*append,
                message: list.failure_message(*append),
                detail: format!("{}: {detail}", request.as_deref().unwrap_or(list.request())),
                retry: IndexListActionDisplay {
                    id: list.element_id("retry"),
                    label: "Try again",
                    a11y_label: list.retry_a11y_label(),
                    available: true,
                },
            }),
        }
    }
}

/// The `MusicIndex` name list of a destination list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexNameList {
    /// `GET /v1/feeds/by-name`, for the Releases list.
    Releases,
    /// `GET /v1/artist-names`, for the Artists list.
    Artists,
}

impl IndexNameList {
    const fn noun(self) -> &'static str {
        match self {
            Self::Releases => "releases",
            Self::Artists => "artists",
        }
    }

    const fn singular(self) -> &'static str {
        match self {
            Self::Releases => "release",
            Self::Artists => "artist",
        }
    }

    /// The request that the error report names.
    const fn request(self) -> &'static str {
        match self {
            Self::Releases => "GET /v1/feeds/by-name",
            Self::Artists => "GET /v1/artist-names",
        }
    }

    fn element_id(self, part: &str) -> String {
        format!("music-index-{}-{part}", self.noun())
    }

    const fn loading_label(self) -> &'static str {
        match self {
            Self::Releases => "Loading MusicIndex releases",
            Self::Artists => "Loading MusicIndex artists",
        }
    }

    fn failure_message(self, append: bool) -> String {
        let (noun, singular) = (self.noun(), self.singular());
        if append {
            format!(
                "MusicIndex did not answer the request for more {noun}. The list keeps the loaded {noun}."
            )
        } else {
            format!(
                "MusicIndex did not answer the {singular} list request. The list shows Library {noun} only."
            )
        }
    }

    fn retry_a11y_label(self) -> String {
        format!("Send the MusicIndex {} list request again", self.singular())
    }

    fn load_more_row(self, available: bool) -> IndexListStatusRowVm {
        IndexListStatusRowVm::LoadMore(IndexListActionDisplay {
            id: self.element_id("load-more"),
            label: "Load more",
            a11y_label: format!("Load more MusicIndex {}", self.noun()),
            available,
        })
    }
}

/// One action of a status row, with its typed availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexListActionDisplay {
    /// Stable element id.
    pub(crate) id: String,
    /// The visible label.
    pub(crate) label: &'static str,
    /// The accessibility label.
    pub(crate) a11y_label: String,
    /// Whether the action can run now.
    pub(crate) available: bool,
}

/// The status row at the end of a list in the scope "Everything" (packet
/// 016 decision D8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IndexListStatusRowVm {
    /// The request of page 1 is in flight.
    Loading {
        element_id: String,
        label: &'static str,
    },
    /// A request failed. `retry` sends it again. `first_page` is true for
    /// a failure of page 1.
    Failed {
        element_id: String,
        first_page: bool,
        message: String,
        detail: String,
        retry: IndexListActionDisplay,
    },
    /// `MusicIndex` has more pages. While a request for the next page is in
    /// flight, the action is not available.
    LoadMore(IndexListActionDisplay),
}

impl IndexListStatusRowVm {
    /// Whether the row shows before the item rows. The loading row and the
    /// error row of page 1 show first, so a long Library list does not
    /// hide them. The other rows show after the last item row.
    #[must_use]
    pub(crate) const fn at_list_start(&self) -> bool {
        match self {
            Self::Loading { .. } => true,
            Self::Failed { first_page, .. } => *first_page,
            Self::LoadMore(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(rows: &[&str], cursor: Option<&str>, has_more: bool) -> IndexNameListBatch<String> {
        IndexNameListBatch {
            rows: rows.iter().map(|row| (*row).to_owned()).collect(),
            cursor: cursor.map(str::to_owned),
            has_more,
        }
    }

    /// R16-06: the sort key of Stophammer ADR 0070 §1.
    #[test]
    fn adr_0084_scopes_r16_06_name_sort_key() {
        assert_eq!(name_sort_key("$2 Holla"), "$2 holla");
        assert_eq!(name_sort_key("  The   Ohm "), "the ohm");
        assert_eq!(name_sort_key("The Ohm!"), "the ohm!");
        assert_eq!(name_sort_key("ÉCLAIR\tNoir"), "éclair noir");
    }

    /// R16-07: an append keeps the rows, and `has_more` and the cursor
    /// follow the batch.
    #[test]
    fn adr_0084_scopes_r16_07_append_keeps_rows() {
        let mut page = IndexNameListPage::<String>::default();
        assert!(page.is_not_started());
        let first = page.begin_load(false).unwrap();
        assert_eq!(first.cursor, None);
        assert!(page.first_page_in_flight());
        assert!(page.finish_load(batch(&["a", "b"], Some("c1"), true), first.generation));
        assert!(page.has_more());

        let next = page.begin_load(true).unwrap();
        assert_eq!(next.cursor.as_deref(), Some("c1"));
        assert!(page.begin_load(true).is_none(), "one append at a time");
        assert!(page.finish_load(batch(&["c"], None, false), next.generation));
        assert_eq!(page.rows(), ["a", "b", "c"]);
        assert!(!page.has_more());
        assert!(page.begin_load(true).is_none(), "no more pages");
    }

    /// R16-07: a finish and a failure of an older generation change
    /// nothing.
    #[test]
    fn adr_0084_scopes_r16_07_old_generation_changes_nothing() {
        let mut page = IndexNameListPage::<String>::default();
        let old = page.begin_load(false).unwrap();
        let new = page.begin_load(false).unwrap();
        assert!(new.generation > old.generation);

        assert!(!page.finish_load(batch(&["stale"], Some("x"), true), old.generation));
        assert!(!page.fail_load("stale", old.generation));
        assert!(page.rows().is_empty());
        assert!(page.first_page_in_flight());

        assert!(page.finish_load(batch(&["fresh"], None, false), new.generation));
        assert_eq!(page.rows(), ["fresh"]);
    }

    /// R16-12 and decision D8: the status rows of each load state.
    #[test]
    fn adr_0084_scopes_r16_12_status_rows() {
        let list = IndexNameList::Artists;
        let mut page = IndexNameListPage::<String>::default();
        assert_eq!(page.status_row(list), None);

        let first = page.begin_load(false).unwrap();
        assert_eq!(
            page.status_row(list),
            Some(IndexListStatusRowVm::Loading {
                element_id: "music-index-artists-loading".to_owned(),
                label: "Loading MusicIndex artists",
            })
        );

        assert!(page.fail_load("connection refused", first.generation));
        let status = page.status_row(list);
        assert!(status
            .as_ref()
            .is_some_and(IndexListStatusRowVm::at_list_start));
        let Some(IndexListStatusRowVm::Failed {
            message,
            detail,
            retry,
            ..
        }) = status
        else {
            panic!("a failure on page 1 gives the error row");
        };
        assert_eq!(
            message,
            "MusicIndex did not answer the artist list request. The list shows Library artists only."
        );
        assert_eq!(detail, "GET /v1/artist-names: connection refused");
        assert_eq!(retry.label, "Try again");
        assert!(retry.available);

        let again = page.begin_retry().unwrap();
        assert_eq!(again.cursor, None, "the retry of page 1 asks for page 1");
        assert!(page.finish_load(batch(&["a"], Some("c1"), true), again.generation));
        assert_eq!(
            page.status_row(list),
            Some(IndexListStatusRowVm::LoadMore(IndexListActionDisplay {
                id: "music-index-artists-load-more".to_owned(),
                label: "Load more",
                a11y_label: "Load more MusicIndex artists".to_owned(),
                available: true,
            }))
        );

        let next = page.begin_load(true).unwrap();
        assert!(matches!(
            page.status_row(list),
            Some(IndexListStatusRowVm::LoadMore(IndexListActionDisplay {
                available: false,
                ..
            }))
        ));
        assert!(page.fail_load("timeout", next.generation));
        assert_eq!(page.rows(), ["a"], "the loaded rows stay");
        let status = page.status_row(list);
        assert!(!status
            .as_ref()
            .is_some_and(IndexListStatusRowVm::at_list_start));
        let Some(IndexListStatusRowVm::Failed { message, .. }) = status else {
            panic!("the error row replaces Load more");
        };
        assert!(message.contains("more artists"));
        let retry = page.begin_retry().unwrap();
        assert_eq!(
            retry.cursor.as_deref(),
            Some("c1"),
            "the retry asks for the same page"
        );
    }

    /// R16-09 and decision D7: the watermark.
    #[test]
    fn adr_0084_scopes_r16_09_watermark() {
        assert!(watermark_shows(&"b", Some(&"c"), true));
        assert!(watermark_shows(&"c", Some(&"c"), true));
        assert!(!watermark_shows(&"d", Some(&"c"), true));
        assert!(!watermark_shows(&"a", None, true));
        assert!(watermark_shows(&"z", Some(&"c"), false));
        assert!(watermark_shows(&"z", None, false));
    }

    /// R16-11: the control exists for Releases and Artists only, with the
    /// labels of decision D2. The default scope is `Library`.
    #[test]
    fn adr_0084_scopes_r16_11_scope_control() {
        assert_eq!(MusicListScope::default(), MusicListScope::Library);
        for destination in MusicDestination::ALL {
            let display =
                MusicScopeControlDisplay::for_destination(destination, MusicListScope::Library);
            assert_eq!(
                display.is_some(),
                matches!(
                    destination,
                    MusicDestination::Releases | MusicDestination::Artists
                ),
                "{destination:?}"
            );
        }
        let releases = MusicScopeControlDisplay::for_destination(
            MusicDestination::Releases,
            MusicListScope::Everything,
        )
        .unwrap();
        assert_eq!(releases.selected, MusicListScope::Everything);
        assert_eq!(
            releases
                .options
                .map(|option| (option.label, option.a11y_label)),
            [
                ("Library", "Show Library releases only"),
                ("Everything", "Show Library and MusicIndex releases"),
            ]
        );
        let artists = MusicScopeControlDisplay::for_destination(
            MusicDestination::Artists,
            MusicListScope::Library,
        )
        .unwrap();
        assert_eq!(
            artists
                .options
                .map(|option| (option.scope, option.a11y_label)),
            [
                (MusicListScope::Library, "Show Library artists only"),
                (
                    MusicListScope::Everything,
                    "Show Library and MusicIndex artists"
                ),
            ]
        );
    }

    /// R16-12 for the Releases list: the texts of the loading row, the
    /// error row and "Load more" name the release list.
    #[test]
    fn adr_0084_scopes_r16_12_release_status_rows() {
        let list = IndexNameList::Releases;
        let mut page = IndexNameListPage::<String>::default();
        let first = page.begin_load(false).unwrap();
        assert!(matches!(
            page.status_row(list),
            Some(IndexListStatusRowVm::Loading {
                label: "Loading MusicIndex releases",
                ..
            })
        ));
        assert!(page.fail_load("connection refused", first.generation));
        let Some(IndexListStatusRowVm::Failed {
            message, detail, ..
        }) = page.status_row(list)
        else {
            panic!("a failure on page 1 gives the error row");
        };
        assert_eq!(
            message,
            "MusicIndex did not answer the release list request. The list shows Library releases only."
        );
        assert_eq!(detail, "GET /v1/feeds/by-name: connection refused");
        let again = page.begin_retry().unwrap();
        assert!(page.finish_load(batch(&["a"], Some("c1"), true), again.generation));
        let Some(IndexListStatusRowVm::LoadMore(load_more)) = page.status_row(list) else {
            panic!("more pages give Load more");
        };
        assert_eq!(load_more.a11y_label, "Load more MusicIndex releases");
    }
}
