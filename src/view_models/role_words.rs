//! The role words of a named feed link (ADR 0085 Decision 3).
//!
//! [ADR 0082 task 003](../../docs/tasks/archive/adr-0082-task-003-release-page-named-feeds.md)
//! adds this module for the role row of a release page. The
//! [ADR 0084 phase plan](../../docs/plans/adr-0084-music-column-browser-phase-plan.md)
//! packet 008 uses it for the "As <roles>" headings and the "Role not
//! confirmed" rows of a feed page.
//!
//! The app keeps no list of role words. "label" and "recordLabel" are two
//! different words.

#![warn(clippy::pedantic)]

/// The separator between the two sides of a stated-sides text.
pub(crate) const SIDE_SEPARATOR: &str = " \u{b7} ";
/// The label of the album side of a stated-sides text.
pub(crate) const ALBUM_SIDE_LABEL: &str = "the album";
/// The word a stated-sides side shows when it states no role.
const NO_ROLE_LABEL: &str = "no role";

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Splits `role_set` at `,`, trims each item and drops each empty item. It
/// keeps the case of the feed. It removes an exact duplicate. It sorts by
/// the lowercase word, then by the word.
#[must_use]
pub(crate) fn role_words(role_set: Option<&str>) -> Vec<String> {
    let Some(role_set) = role_set else {
        return Vec::new();
    };
    let mut words: Vec<String> = role_set
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    words.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    words.dedup();
    words
}

/// The role word that confirms an artist link (ADR 0085 Decisions 2 and
/// 6).
const ARTIST_ROLE: &str = "artist";

/// Tells if `role_set`'s role words contain `artist`, without case (ADR
/// 0085 Decisions 2 and 6). `named_feeds::select_artist_feed` and the feed
/// page level and frame title share this one test.
#[must_use]
pub(crate) fn role_contains_artist(role_set: Option<&str>) -> bool {
    role_words(role_set)
        .iter()
        .any(|word| word.eq_ignore_ascii_case(ARTIST_ROLE))
}

/// Joins `words`: "label", "label and producer", "distributor, label and
/// producer".
#[must_use]
pub(crate) fn role_list_text(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [only] => only.clone(),
        _ => {
            let (last, rest) = words.split_last().expect("words is not empty");
            format!("{} and {last}", rest.join(", "))
        }
    }
}

/// Writes the first letter of `text` with a capital letter: "Label and
/// producer".
#[must_use]
pub(crate) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The text of each side of a named feed link, by title (ADR 0085 Decision
/// 3). The side that states a role comes first. When both sides state a
/// role, the feed side comes first. A side with no role gives "no role".
///
/// Examples: "Sir Libre Records: label \u{b7} the album: recordLabel" and
/// "The album: label \u{b7} Sir Libre Records: no role".
#[must_use]
pub(crate) fn stated_sides_text(
    feed_title: &str,
    feed_rel: Option<&str>,
    album_rel: Option<&str>,
) -> String {
    let feed_part = format!(
        "{feed_title}: {}",
        nonempty(feed_rel).unwrap_or(NO_ROLE_LABEL)
    );
    let album_part = format!(
        "{ALBUM_SIDE_LABEL}: {}",
        nonempty(album_rel).unwrap_or(NO_ROLE_LABEL)
    );
    // Only the album label gets a capital letter when it comes first. The
    // feed title keeps the case that the feed states.
    if nonempty(feed_rel).is_none() && nonempty(album_rel).is_some() {
        format!(
            "{}{SIDE_SEPARATOR}{feed_part}",
            capitalize_first(&album_part)
        )
    } else {
        format!("{feed_part}{SIDE_SEPARATOR}{album_part}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R85-8-01: `role_contains_artist` finds "artist" without case, and
    /// keeps "label" and "recordLabel" (not "artist") out.
    #[test]
    fn adr_0085_feed_page_role_contains_artist_reads_without_case() {
        assert!(role_contains_artist(Some("artist, producer")));
        assert!(role_contains_artist(Some("Producer, ARTIST")));
        assert!(!role_contains_artist(Some("label")));
        assert!(!role_contains_artist(Some("label, recordLabel")));
        assert!(!role_contains_artist(None));
    }

    /// R85-3-10: `role_words` keeps the feed case, removes an exact
    /// duplicate and sorts. "label" and "recordLabel" stay two words.
    /// `role_list_text` gives "label", "label and producer" and
    /// "distributor, label and producer".
    #[test]
    fn adr_0085_named_feeds_role_words_keeps_case_and_sorts() {
        assert_eq!(
            role_words(Some("Producer, label, producer, Label")),
            vec![
                "Label".to_owned(),
                "label".to_owned(),
                "Producer".to_owned(),
                "producer".to_owned(),
            ]
        );
        assert_eq!(
            role_words(Some("label, recordLabel")),
            vec!["label".to_owned(), "recordLabel".to_owned()]
        );
        assert_eq!(
            role_words(Some(" label , , producer ")),
            vec!["label".to_owned(), "producer".to_owned()]
        );
        assert_eq!(role_words(None), Vec::<String>::new());

        assert_eq!(role_list_text(&["label".to_owned()]), "label");
        assert_eq!(
            role_list_text(&["label".to_owned(), "producer".to_owned()]),
            "label and producer"
        );
        assert_eq!(
            role_list_text(&[
                "distributor".to_owned(),
                "label".to_owned(),
                "producer".to_owned()
            ]),
            "distributor, label and producer"
        );
        assert_eq!(role_list_text(&[]), "");
    }

    /// R85-3-10: `capitalize_first` writes the first letter with a capital
    /// letter.
    #[test]
    fn adr_0085_named_feeds_capitalize_first_writes_one_capital_letter() {
        assert_eq!(capitalize_first("label and producer"), "Label and producer");
        assert_eq!(capitalize_first(""), "");
    }

    /// R85-3-11: `stated_sides_text` gives "Sir Libre Records: label \u{b7}
    /// the album: recordLabel" and "The album: label \u{b7} Sir Libre
    /// Records: no role".
    #[test]
    fn adr_0085_named_feeds_stated_sides_text_names_each_side() {
        assert_eq!(
            stated_sides_text("Sir Libre Records", Some("label"), Some("recordLabel")),
            "Sir Libre Records: label \u{b7} the album: recordLabel"
        );
        assert_eq!(
            stated_sides_text("Sir Libre Records", None, Some("label")),
            "The album: label \u{b7} Sir Libre Records: no role"
        );
    }

    /// R85-3-11: a feed title keeps the case that the feed states. Only the
    /// album label gets a capital letter, when it comes first.
    #[test]
    fn adr_0085_named_feeds_stated_sides_text_keeps_the_feed_title_case() {
        assert_eq!(
            stated_sides_text("heyCitizen", Some("label"), None),
            "heyCitizen: label \u{b7} the album: no role"
        );
        assert_eq!(
            stated_sides_text("heyCitizen", None, Some("label")),
            "The album: label \u{b7} heyCitizen: no role"
        );
    }
}
