//! The release sections of an artist page (ADR 0085 Decision 3, ADR 0084
//! Decision 7, packet 017).
//!
//! The feed page and the name level page show their releases through this
//! one composite. It draws each section heading, the section note, the
//! status of the `MusicIndex` part, and each release row. The view model
//! decides each text, each mark and each open action. This composite only
//! lays them out with named tokens.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClickEvent, Image, IntoElement, Modifiers, ParentElement, ScrollHandle,
    SharedString, Styled, Window,
};

use crate::ui::composites::music_column_scroll::ScrollReveal;
use crate::ui::composites::release_row_menu::{release_row_menu, ReleaseRowMenuContext};
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, TagBadge, TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::primitives::{Checkbox, CheckboxState, Label, LabelVariant, SectionHeader};
use crate::ui::tokens::{color, Radius, SemanticColor, Spacing};
use crate::view_models::list_selection::RowCheckState;
use crate::view_models::name_level_page::ReleaseListStatus;
use crate::view_models::publisher_page::{
    PublisherPageAlbumVm, PublisherPageSectionVm, TitleDisplay,
};
use crate::view_models::workspace::ReleaseKey;

/// A click on a release row (ADR 0085 Decision 7 design, ADR 0084 packet
/// 018). `label` is the display title of the release. The app layer opens
/// the release page, or changes the selection in selection mode. The
/// composite builds no navigation of its own.
pub(crate) type ClickReleaseCallback =
    Rc<dyn Fn(&ReleaseKey, String, Modifiers, &mut Window, &mut App)>;

/// A click on the checkbox of a release row (ADR 0084 packet 018).
pub(crate) type ToggleReleaseCallback = Rc<dyn Fn(&ReleaseKey, &mut Window, &mut App)>;

/// The release rows of a page in the Music detail column, with their
/// selection facts (ADR 0084 Decision 10, packet 018). The app layer sets
/// `PublisherPageAlbumVm::selection` on each row of `sections`.
pub(crate) struct ReleaseSectionRows {
    /// The sections of the page, in their visible order.
    pub(crate) sections: Vec<PublisherPageSectionVm>,
    /// The held scroll handle of the detail column.
    pub(crate) scroll: ScrollHandle,
    /// The request to keep the focused row in view.
    pub(crate) reveal: ScrollReveal,
    /// A click on the body of a release row.
    pub(crate) on_click_release: ClickReleaseCallback,
    /// A click on the checkbox of a release row.
    pub(crate) on_toggle_release: ToggleReleaseCallback,
    /// The context of the release row menus (ADR 0084 task 031). The app
    /// layer sets `PublisherPageAlbumVm::menu` on each row.
    pub(crate) menu: ReleaseRowMenuContext,
}

/// One section, with its heading, its note, its status and its release
/// rows (ADR 0085 Decision 3, ADR 0084 Decision 7).
#[must_use]
pub(crate) fn render_release_section(
    section: &PublisherPageSectionVm,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    rows: &ReleaseSectionRows,
    cx: &App,
) -> AnyElement {
    let mut body = div()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .child(SectionHeader::new(section.heading.clone()));
    if let Some(note) = section.note {
        body = body.child(
            Label::new(note)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    if let Some(status) = &section.status {
        body = body.child(render_release_list_status(status, cx));
    }
    body.child(
        div()
            .flex()
            .flex_col()
            .gap(Spacing::XXS.scaled(cx))
            .children(
                section
                    .rows
                    .iter()
                    .map(|row| render_release_row(row, &section.heading, album_thumbs, rows, cx)),
            ),
    )
    .into_any_element()
}

/// The status of the `MusicIndex` part of a release list. A failure shows
/// its report first, and its technical detail after it.
#[must_use]
pub(crate) fn render_release_list_status(status: &ReleaseListStatus, cx: &App) -> AnyElement {
    match status {
        ReleaseListStatus::Loading(text) | ReleaseListStatus::MoreAvailable(text) => {
            Label::new(*text)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel)
                .into_any_element()
        }
        ReleaseListStatus::Failed { report, detail } => div()
            .flex()
            .flex_col()
            .gap(Spacing::XXS.scaled(cx))
            .p(Spacing::SM.scaled(cx))
            .rounded(Radius::MD.scaled(cx))
            .border_1()
            .border_color(color(cx, SemanticColor::Separator))
            .child(
                Label::new(*report)
                    .variant(LabelVariant::Body)
                    .color(SemanticColor::WarningLabel),
            )
            .child(
                Label::new(detail.clone())
                    .variant(LabelVariant::Caption)
                    .color(SemanticColor::TertiaryLabel),
            )
            .into_any_element(),
    }
}

fn render_release_row(
    row: &PublisherPageAlbumVm,
    section_heading: &str,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    rows: &ReleaseSectionRows,
    cx: &App,
) -> AnyElement {
    let (title, missing_note) = title_display_text(&row.title);
    let row_id = row
        .feed_guid
        .clone()
        .unwrap_or_else(|| format!("no-guid:{title}"));
    let thumb_image = row
        .image_url
        .as_deref()
        .and_then(|url| album_thumbs.get(url))
        .and_then(Clone::clone);

    // Column text does not call `truncate()` (ADR 0063,
    // docs/troubleshooting/column-text-truncation.md). `overflow_hidden()`
    // is the required mitigation for text stacked in a flex column.
    let mut text_block = div()
        .flex()
        .flex_col()
        .min_w_0()
        .overflow_hidden()
        .gap(Spacing::XXS.scaled(cx))
        .child(Label::new(title.clone()).variant(LabelVariant::Headline));
    if let Some(missing_note) = missing_note {
        text_block = text_block.child(
            Label::new(missing_note)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    if let Some(artist) = &row.artist {
        text_block =
            text_block.child(Label::new(artist.display_text()).variant(LabelVariant::Caption));
    }
    // ADR 0085 Decision 3: a "Role not confirmed" row names who stated
    // which role. ADR 0084 Decision 7: a "Matched by name" row tells that
    // the relation is inferred. A row in each other section shows no
    // detail line: its section already tells the relation.
    if let Some(detail_line) = &row.detail_line {
        text_block = text_block.child(
            Label::new(detail_line.clone())
                .variant(LabelVariant::Micro)
                .color(SemanticColor::WarningLabel),
        );
    }

    // ADR 0084 packet 018: the selection facts of the row come from the
    // view model. The checkbox shows only in selection mode.
    let selection = &row.selection;
    let mut list_row = ListRow::new(SharedString::from(row.element_id(section_heading)))
        .selected(selection.is_marked())
        .focused(selection.focused)
        .scroll_probe(selection.focused.then(|| rows.reveal.probe()));
    if let (true, Some(open)) = (selection.checkbox_visible, &row.open) {
        let on_toggle_release = Rc::clone(&rows.on_toggle_release);
        let key = open.key.clone();
        list_row = list_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("publisher-album-checkbox:{row_id}")),
                checkbox_state_for(selection.check_state),
            )
            .a11y_label(selection.checkbox_a11y_label.clone())
            .on_click(move |_event: &ClickEvent, window, cx| {
                cx.stop_propagation();
                on_toggle_release(&key, window, cx);
            })
            .into_any_element(),
        );
    }
    list_row = list_row
        .child(Thumbnail::new(EntityKind::Release, ThumbnailSize::Sm).image(thumb_image))
        .child(div().flex_1().min_w_0().child(text_block));

    // The view model decides the open action's availability. A row with
    // no open action gets no click handler, so it stays inert.
    if let Some(open) = &row.open {
        let on_click_release = Rc::clone(&rows.on_click_release);
        let key = open.key.clone();
        let label = title.clone();
        list_row = list_row
            .a11y_label(ListRowA11yLabel {
                label: open.a11y_label.clone().into(),
            })
            .on_click(move |event: &ClickEvent, window, cx| {
                on_click_release(&key, label.clone(), event.modifiers(), window, cx);
            });
    }

    let mut marks = Vec::new();
    if row.not_listed {
        marks.push(PublisherPageAlbumVm::NOT_LISTED_LABEL);
    }
    if row.in_library {
        marks.push(PublisherPageAlbumVm::IN_LIBRARY_LABEL);
    }
    if !marks.is_empty() {
        let mut trailing = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx));
        for mark in marks {
            trailing = trailing.child(TagBadge::new(TagBadgeDisplay {
                kind: EntityKind::Generic,
                label: Some(SharedString::from(mark)),
            }));
        }
        list_row = list_row.child(trailing);
    }

    // ADR 0084 task 031: the one release row menu, the same as a row of
    // the Releases list. The view model gives `None` in selection mode.
    list_row
        .menu_cell(release_row_menu(row.menu.as_ref(), &rows.menu))
        .into_any_element()
}

/// Maps the typed check state of the view model to the state of the
/// checkbox primitive. A release row has one member, so it never reads
/// mixed. The singles group page uses it for its `MusicIndex` rows.
#[must_use]
pub(crate) fn checkbox_state_for(check_state: RowCheckState) -> CheckboxState {
    match check_state {
        RowCheckState::Checked => CheckboxState::Checked,
        RowCheckState::Mixed => CheckboxState::Mixed,
        RowCheckState::Clear => CheckboxState::NotChecked,
    }
}

/// The visible title of a row, and the "No title" note when the release
/// states no title.
#[must_use]
pub(crate) fn title_display_text(title: &TitleDisplay) -> (String, Option<&'static str>) {
    match title {
        TitleDisplay::Stated(text) => (text.clone(), None),
        TitleDisplay::Missing(guid) => (guid.clone(), Some(TitleDisplay::MISSING_LABEL)),
    }
}
