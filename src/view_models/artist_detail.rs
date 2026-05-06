//! Shared artist detail page contract.
//!
//! This module owns the GPUI-free page shape consumed by the artist shell.
//! Screens and context-specific VMs provide images, row click handlers, and
//! optional sections through shell slots.

#![warn(clippy::pedantic)]

/// Page-level projection for an artist detail surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtistDetailPageVm {
    pub title: String,
    pub subtitle: Option<String>,
    pub detail_rows: Vec<ArtistDetailFactVm>,
    pub shows_feed_section: bool,
}

impl ArtistDetailPageVm {
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        subtitle: Option<impl Into<String>>,
        detail_rows: Vec<ArtistDetailFactVm>,
        shows_feed_section: bool,
    ) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.map(Into::into),
            detail_rows,
            shows_feed_section,
        }
    }
}

/// Display-ready fact row for an artist detail page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtistDetailFactVm {
    pub key: String,
    pub value: String,
    pub max_lines: usize,
}

impl ArtistDetailFactVm {
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<String>, max_lines: usize) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            max_lines,
        }
    }
}
