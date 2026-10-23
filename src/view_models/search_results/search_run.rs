//! The run of one search query (ADR 0087 Decisions 1, 5 and 6, packet 002).
//!
//! Each search run parses its text one time. The typed query decides the
//! parts that run: the Library part runs for the scopes `Everything` and
//! `LibraryOnly`. The Index part sends a request only when the Index can
//! apply each term. The `q` of that request holds only the words and the
//! phrases of the query (ADR 0087 invariant "No Index request carries field
//! syntax in `q`"). Only [`IndexSearchPlan::for_query`] builds an
//! [`IndexSearchRequest`].
//!
//! When the Index part sends no request, the plan names the cause. The
//! search page then shows the note of Decision 5.
//!
//! The module has no GPUI type and no I/O.

#![warn(clippy::pedantic)]

use crate::db::TrackRow;
use crate::view_models::search_query::{QueryScope, QueryTerm, SearchQuery, TermKind};

/// The text of the note under the Library rows when the Index part sends no
/// request because the Index cannot apply a term (ADR 0087 Decision 5).
pub(crate) const INDEX_SKIP_ROWS_TEXT: &str = "The Library rows show.";

/// The title of the empty state of a query whose scopes cannot apply
/// together.
pub(crate) const SCOPE_CONFLICT_TITLE: &str = "No results";

/// The text of the empty state of a query whose scopes cannot apply
/// together.
pub(crate) const SCOPE_CONFLICT_TEXT: &str =
    "The search asks for the Library only and for MusicIndex only. Remove one of the fields.";

/// The run of one query text: the trimmed text, the typed query and the
/// Index plan.
#[derive(Clone, Debug)]
pub(crate) struct SearchRun {
    text: String,
    query: SearchQuery,
    index_plan: IndexSearchPlan,
}

impl SearchRun {
    /// Trims `text`, parses it one time and computes the Index plan.
    #[must_use]
    pub(crate) fn new(text: &str) -> SearchRun {
        let text = text.trim().to_owned();
        let query = SearchQuery::parse(&text);
        let index_plan = IndexSearchPlan::for_query(&query);
        SearchRun {
            text,
            query,
            index_plan,
        }
    }

    /// The trimmed query text.
    #[must_use]
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The typed query of the text.
    #[must_use]
    pub(crate) fn query(&self) -> &SearchQuery {
        &self.query
    }

    /// The plan of the Index part.
    #[must_use]
    pub(crate) fn index_plan(&self) -> &IndexSearchPlan {
        &self.index_plan
    }

    /// `true` when the query holds a term that can run. A run that is not
    /// runnable opens no page and sends nothing (packet 002 decision D19).
    #[must_use]
    pub(crate) fn is_runnable(&self) -> bool {
        self.query.has_runnable_content()
    }

    /// `true` when the Library part runs (packet 002 decision D15).
    #[must_use]
    pub(crate) fn runs_library(&self) -> bool {
        library_part_runs(&self.query)
    }
}

/// `true` when the Library part of `query` runs: the query is runnable, and
/// its scope is `Everything` or `LibraryOnly` (packet 002 decision D15).
#[must_use]
pub(crate) fn library_part_runs(query: &SearchQuery) -> bool {
    query.has_runnable_content()
        && matches!(
            query.scope(),
            QueryScope::Everything | QueryScope::LibraryOnly
        )
}

/// The plan of the Index part of one query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IndexSearchPlan {
    /// Send the Index search with this request.
    Send(IndexSearchRequest),
    /// Send no Index request, for this cause.
    Skip(IndexSkip),
}

impl IndexSearchPlan {
    /// The plan of `query` (ADR 0087 Decision 5).
    #[must_use]
    pub(crate) fn for_query(query: &SearchQuery) -> IndexSearchPlan {
        match query.scope() {
            QueryScope::Conflict => return IndexSearchPlan::Skip(IndexSkip::Conflict),
            QueryScope::LibraryOnly => return IndexSearchPlan::Skip(IndexSkip::LibraryOnly),
            QueryScope::Everything | QueryScope::IndexOnly => {}
        }
        let filters = query.index_filter_terms();
        if !filters.is_empty() {
            return IndexSearchPlan::Skip(IndexSkip::Unsupported(
                UnsupportedIndexTerms::from_terms(&filters),
            ));
        }
        match query.index_q() {
            Some(q) => IndexSearchPlan::Send(IndexSearchRequest { q }),
            None => IndexSearchPlan::Skip(IndexSkip::NoIndexPart),
        }
    }
}

/// The request of one Index search. Only [`IndexSearchPlan::for_query`]
/// builds it, so `q` holds no field syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexSearchRequest {
    /// The words and the phrases of the query.
    pub(crate) q: String,
}

/// The cause of an Index part that sends no request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IndexSkip {
    /// The query has no word and no phrase for `q`.
    NoIndexPart,
    /// The query asks for the Library only.
    LibraryOnly,
    /// The query asks for the Library only and for the Index only.
    Conflict,
    /// The Index cannot apply one or more terms of the query.
    Unsupported(UnsupportedIndexTerms),
}

/// The terms that the Index cannot apply yet (ADR 0087 Decision 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UnsupportedIndexTerms {
    /// Each distinct field token, in text order, for example `after:`.
    pub(crate) field_tokens: Vec<&'static str>,
    /// `true` when the query has an excluded word or phrase.
    pub(crate) excludes_words: bool,
}

impl UnsupportedIndexTerms {
    /// The unsupported terms of `terms`, the result of
    /// `SearchQuery::index_filter_terms`.
    fn from_terms(terms: &[&QueryTerm]) -> Self {
        let mut field_tokens = Vec::new();
        let mut excludes_words = false;
        for term in terms {
            match &term.kind {
                TermKind::Field(field) => {
                    let token = field.name.info().token;
                    if !field_tokens.contains(&token) {
                        field_tokens.push(token);
                    }
                }
                TermKind::Word(_) | TermKind::Phrase(_) => excludes_words = true,
            }
        }
        Self {
            field_tokens,
            excludes_words,
        }
    }

    /// The title of the note, for example "`MusicIndex` cannot filter by
    /// after: and kind: yet" (packet 002 "The note of Decision 5").
    #[must_use]
    pub(crate) fn note_title(&self) -> String {
        let tokens = token_list(&self.field_tokens);
        match (tokens.is_empty(), self.excludes_words) {
            (true, true) => "MusicIndex cannot exclude words yet".to_owned(),
            (false, true) => format!("MusicIndex cannot filter by {tokens} or exclude words yet"),
            (false, false) => format!("MusicIndex cannot filter by {tokens} yet"),
            (true, false) => "MusicIndex cannot apply this search yet".to_owned(),
        }
    }
}

/// Joins the tokens: `a`, `a and b`, or `a, b and c`.
fn token_list(tokens: &[&str]) -> String {
    match tokens {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

/// The Library rows of one run (packet 002 "The Library facts from the
/// tree").
#[derive(Clone, Debug, Default)]
pub(crate) struct LibrarySearchRows {
    /// The rows of the Releases tab and the Artists tab.
    pub(crate) releases: Vec<TrackRow>,
    /// The rows of the Tracks tab.
    pub(crate) tracks: Vec<TrackRow>,
    /// The row count of SQL before the date split (packet 002 decision
    /// D22).
    pub(crate) matched_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(text: &str) -> IndexSearchPlan {
        IndexSearchPlan::for_query(&SearchQuery::parse(text))
    }

    fn unsupported(field_tokens: Vec<&'static str>, excludes_words: bool) -> IndexSearchPlan {
        IndexSearchPlan::Skip(IndexSkip::Unsupported(UnsupportedIndexTerms {
            field_tokens,
            excludes_words,
        }))
    }

    /// R87-2-21: a Library-only field gives `Skip(LibraryOnly)`, also with
    /// `-`. `in:index` with a Library-only field gives `Skip(Conflict)`.
    #[test]
    fn adr_0087_library_fields_r87_2_21_scope_skips_the_index() {
        for text in [
            "haleen is:ready",
            "haleen -has:split",
            "haleen playlist:x",
            "in:library haleen",
        ] {
            assert_eq!(
                plan(text),
                IndexSearchPlan::Skip(IndexSkip::LibraryOnly),
                "R87-2-21: {text}"
            );
        }
        assert_eq!(
            plan("in:index is:ready"),
            IndexSearchPlan::Skip(IndexSkip::Conflict)
        );
    }

    /// R87-2-22: an Index filter term gives `Skip(Unsupported)` with each
    /// distinct token in text order. `note_title` gives each text of the
    /// note of Decision 5.
    #[test]
    fn adr_0087_library_fields_r87_2_22_unsupported_terms_and_note_titles() {
        assert_eq!(
            plan("haleen after:2026-10-01"),
            unsupported(vec!["after:"], false)
        );
        assert_eq!(
            plan("kind:single after:2026-10 kind:ep"),
            unsupported(vec!["kind:", "after:"], false)
        );
        assert_eq!(plan("haleen -remix"), unsupported(Vec::new(), true));

        let title = |tokens: Vec<&'static str>, excludes_words| {
            UnsupportedIndexTerms {
                field_tokens: tokens,
                excludes_words,
            }
            .note_title()
        };
        assert_eq!(
            title(vec!["after:"], false),
            "MusicIndex cannot filter by after: yet"
        );
        assert_eq!(
            title(vec!["after:", "kind:"], false),
            "MusicIndex cannot filter by after: and kind: yet"
        );
        assert_eq!(
            title(vec!["after:", "kind:", "duration:"], false),
            "MusicIndex cannot filter by after:, kind: and duration: yet"
        );
        assert_eq!(
            title(Vec::new(), true),
            "MusicIndex cannot exclude words yet"
        );
        assert_eq!(
            title(vec!["after:"], true),
            "MusicIndex cannot filter by after: or exclude words yet"
        );
        assert_eq!(
            title(vec!["after:", "kind:"], true),
            "MusicIndex cannot filter by after: and kind: or exclude words yet"
        );
    }

    /// R87-2-23 (plan part): a query whose Index part has only words and
    /// phrases sends `q` equal to `index_q`. `in:` never reaches `q`.
    #[test]
    fn adr_0087_library_fields_r87_2_23_send_holds_only_words() {
        let query = SearchQuery::parse("haleen \"night drive\"");
        assert_eq!(
            IndexSearchPlan::for_query(&query),
            IndexSearchPlan::Send(IndexSearchRequest {
                q: query.index_q().expect("the query has words"),
            })
        );
        assert_eq!(
            plan("haleen in:index"),
            IndexSearchPlan::Send(IndexSearchRequest {
                q: "haleen".to_owned()
            })
        );
        assert_eq!(plan("is:"), IndexSearchPlan::Skip(IndexSkip::NoIndexPart));
    }

    /// R87-2-28: a saved search keeps its text, and each run parses it
    /// again (ADR 0087 Decision 6).
    #[test]
    fn adr_0087_library_fields_r87_2_28_saved_text_parses_at_each_run() -> anyhow::Result<()> {
        use crate::db::saved_searches::{insert_saved_search, saved_searches, StoredStartTab};

        let conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::db::init_schema(&conn)?;
        crate::db::migrate_schema(&conn)?;
        let text = "artist:jsmovie kind:single -title:remix";

        insert_saved_search(&conn, text, StoredStartTab::Releases, 1_792_000_000)?;
        let loaded = saved_searches(&conn)?;

        let run = SearchRun::new(&loaded[0].query);
        assert_eq!(run.text(), text);
        assert_eq!(run.query(), &SearchQuery::parse(text));
        assert!(run.is_runnable());
        assert!(run.runs_library());
        Ok(())
    }
}
