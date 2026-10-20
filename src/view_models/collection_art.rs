//! The art of a collection in the logo frame (ADR 0084 Decision 12).
//!
//! A collection is a singles group, an artist page or a playlist. This
//! module gives each art choice. It is pure, and it carries no renderer
//! type. The renderer maps [`CollectionKind`] to its entity color, and the
//! image cache composes the mosaic off the UI thread.

#![warn(clippy::pedantic)]

use crate::media::logo_frame::repeat_cells;

/// The most covers that a mosaic holds.
const MOSAIC_COVERS: usize = 4;

/// The kind of a collection. It selects the entity color of the frame when
/// the collection has no cover.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum CollectionKind {
    /// A singles group. It uses the release color.
    SinglesGroup,
    /// An artist feed page or an Artists row. It uses the artist color.
    Artist,
    /// A playlist. It uses the playlist color.
    Playlist,
}

/// The art choice of one collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CollectionArtVm {
    /// The image that the collection gives, as a square cover.
    /// `fallback` is the choice for an image that does not load.
    ProvidedImage {
        url: String,
        fallback: Box<CollectionArtVm>,
    },
    /// One cover, as a square cover.
    SingleCover(String),
    /// The cells of the logo mosaic, after the repeat rule.
    Mosaic([String; 4]),
    /// No cover: the logo in the entity color of the kind.
    EntityColor(CollectionKind),
}

impl CollectionArtVm {
    /// Gives the art choice of one collection.
    ///
    /// `covers` comes newest first, or in collection order. Each URL is
    /// trimmed. An empty URL and a repeated URL are dropped, and four
    /// covers at most stay. A provided image comes first. Then one cover
    /// gives [`Self::SingleCover`], two or more give [`Self::Mosaic`], and
    /// none gives [`Self::EntityColor`].
    #[must_use]
    pub(crate) fn resolve<I, S>(provided: Option<&str>, covers: I, kind: CollectionKind) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let covers = distinct_covers(covers);
        let art = match covers.as_slice() {
            [] => Self::EntityColor(kind),
            [cover] => Self::SingleCover(cover.clone()),
            _ => mosaic_cells(&covers).map_or(Self::EntityColor(kind), Self::Mosaic),
        };
        match provided.map(str::trim).filter(|url| !url.is_empty()) {
            Some(url) => Self::ProvidedImage {
                url: url.to_owned(),
                fallback: Box::new(art),
            },
            None => art,
        }
    }

    /// The choice for a provided image that does not load. Each other
    /// choice gives itself.
    #[must_use]
    pub(crate) fn without_provided(&self) -> Self {
        match self {
            Self::ProvidedImage { fallback, .. } => fallback.as_ref().clone(),
            other => other.clone(),
        }
    }
}

/// The repeat rule of the mosaic: two covers give A B / B A, three give
/// A B / C A, and four or more give the first four. Fewer than two covers
/// give `None`.
#[must_use]
pub(crate) fn mosaic_cells(covers: &[String]) -> Option<[String; 4]> {
    repeat_cells(covers)
}

/// Trims each URL, drops each empty URL and each repeated URL, keeps the
/// order, and keeps four at most.
fn distinct_covers<I, S>(covers: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut distinct: Vec<String> = Vec::with_capacity(MOSAIC_COVERS);
    for cover in covers {
        let cover = cover.as_ref().trim();
        if cover.is_empty() || distinct.iter().any(|kept| kept == cover) {
            continue;
        }
        distinct.push(cover.to_owned());
        if distinct.len() == MOSAIC_COVERS {
            break;
        }
    }
    distinct
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// R11-01: the repeat rule, and `None` for fewer than two covers.
    #[test]
    fn adr_0084_logo_frame_r11_01_mosaic_cells() {
        assert_eq!(
            mosaic_cells(&urls(&["a", "b"])),
            Some(["a", "b", "b", "a"].map(str::to_owned))
        );
        assert_eq!(
            mosaic_cells(&urls(&["a", "b", "c"])),
            Some(["a", "b", "c", "a"].map(str::to_owned))
        );
        assert_eq!(
            mosaic_cells(&urls(&["a", "b", "c", "d", "e"])),
            Some(["a", "b", "c", "d"].map(str::to_owned))
        );
        assert_eq!(mosaic_cells(&[]), None);
        assert_eq!(mosaic_cells(&urls(&["a"])), None);
    }

    /// R11-02: an empty URL and a repeated URL are dropped before the
    /// count, so two real covers make a mosaic of two.
    #[test]
    fn adr_0084_logo_frame_r11_02_resolve_drops_empty_and_repeated() {
        let art = CollectionArtVm::resolve(
            None,
            ["a", " ", "", " a ", "b", "a"],
            CollectionKind::Playlist,
        );
        assert_eq!(
            art,
            CollectionArtVm::Mosaic(["a", "b", "b", "a"].map(str::to_owned))
        );
        let one = CollectionArtVm::resolve(None, ["a", "a ", ""], CollectionKind::Playlist);
        assert_eq!(one, CollectionArtVm::SingleCover("a".to_owned()));
    }

    /// R11-03: a provided image wins over four covers, and
    /// `without_provided` gives the mosaic.
    #[test]
    fn adr_0084_logo_frame_r11_03_provided_image_wins() {
        let art = CollectionArtVm::resolve(
            Some(" feed.jpg "),
            ["a", "b", "c", "d"],
            CollectionKind::Artist,
        );
        let mosaic = CollectionArtVm::Mosaic(["a", "b", "c", "d"].map(str::to_owned));
        assert_eq!(
            art,
            CollectionArtVm::ProvidedImage {
                url: "feed.jpg".to_owned(),
                fallback: Box::new(mosaic.clone()),
            }
        );
        assert_eq!(art.without_provided(), mosaic);
        assert_eq!(mosaic.without_provided(), mosaic);
        assert_eq!(
            CollectionArtVm::resolve(Some("  "), ["a"], CollectionKind::Artist),
            CollectionArtVm::SingleCover("a".to_owned())
        );
    }

    /// R11-04: one cover gives `SingleCover`, and no cover gives the entity
    /// color of the collection kind.
    #[test]
    fn adr_0084_logo_frame_r11_04_single_cover_and_entity_color() {
        assert_eq!(
            CollectionArtVm::resolve(None, ["a"], CollectionKind::SinglesGroup),
            CollectionArtVm::SingleCover("a".to_owned())
        );
        for kind in [
            CollectionKind::SinglesGroup,
            CollectionKind::Artist,
            CollectionKind::Playlist,
        ] {
            assert_eq!(
                CollectionArtVm::resolve(None, Vec::<String>::new(), kind),
                CollectionArtVm::EntityColor(kind)
            );
        }
    }
}
