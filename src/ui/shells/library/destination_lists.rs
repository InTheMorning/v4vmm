//! Releases, Artists, and Playlists destination lists (ADR 0084 Decision
//! 2).
//!
//! Each destination reads straight from the Library tree or the loaded
//! playlists; none of these owns a `ContentListPageVm`. Row labels and
//! targets come from the view model; this shell owns only layout and
//! interaction wiring.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, Image, SharedString, Styled,
};
use gpui_component::input::Input;
use gpui_component::Size;

use crate::library::LibraryApp;
use crate::ui::composites::{
    DisclosureSupplementDisplay, DisclosureSupplementLabel, EntityKind, ListRow, ListRowA11yLabel,
    Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::library::{
    DestinationListRowTarget, DestinationListRowVm, PlaylistSidebarRowVm, PlaylistSidebarVm,
};
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

/// Renders the Releases or Artists destination list (ADR 0084 Decision 2).
/// `empty_label` names the row kind for the empty-list message, for
/// example "releases".
pub(crate) fn render_library_destination_rows(
    rows: Vec<DestinationListRowVm>,
    empty_label: &'static str,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut container = div()
        .id("music-destination-list-rows")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .px(spacing::MD)
        .pb(spacing::MD)
        .gap(spacing::XXS);

    if rows.is_empty() {
        container = container.child(
            div()
                .text_center()
                .p(spacing::XXL)
                .text_color(crate::ui::tokens::resolve_color(
                    cx,
                    SemanticColor::TertiaryLabel,
                    None,
                ))
                .child(format!("No {empty_label} yet")),
        );
    } else {
        container = container.children(rows.into_iter().map(|row| {
            let thumbnail = thumbnails.get(&row.element_id).cloned().flatten();
            render_destination_list_row(row, thumbnail, cx)
        }));
    }

    container.into_any_element()
}

fn render_destination_list_row(
    row: DestinationListRowVm,
    thumbnail: Option<Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let entity_kind = match &row.open {
        DestinationListRowTarget::Release(_) => EntityKind::Release,
        DestinationListRowTarget::Artist(_) => EntityKind::Artist,
    };
    let target = row.open.clone();
    let thumbnail_href = row.thumbnail_href.clone();
    let thumb_element_id = format!("{}-thumb", row.element_id);

    ListRow::new(SharedString::from(row.element_id))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label),
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.open_destination_row(&target, cx);
        }))
        .child(render_hoverable_release_thumb(
            thumb_element_id,
            thumbnail_href,
            Thumbnail::new(entity_kind, ThumbnailSize::Sm).image(thumbnail),
            cx,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(row.label)
                        .size(FontSize::Micro)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .when(!row.secondary_text.is_empty(), |el| {
                    el.child(
                        Label::new(row.secondary_text)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    )
                }),
        )
        .into_any_element()
}

/// Wraps a release thumbnail so hovering it animates the matching image
/// (ADR 0084 Decision 2), the same way the Library sidebar tree once did
/// for an album row.
fn render_hoverable_release_thumb(
    element_id: String,
    thumbnail_href: Option<String>,
    thumb: Thumbnail,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let Some(href) = thumbnail_href else {
        return thumb.into_any_element();
    };
    let enter_href = href.clone();
    let leave_href = href;
    div()
        .id(SharedString::from(element_id))
        .on_mouse_move(cx.listener(move |this, _, _, cx| {
            if this.hovered_thumb_url() != Some(enter_href.as_str()) {
                this.set_hovered_thumb(Some(enter_href.clone()), cx);
            }
        }))
        .on_hover(cx.listener(move |this, entered: &bool, _, cx| {
            if !*entered && this.hovered_thumb_url() == Some(leave_href.as_str()) {
                this.set_hovered_thumb(None, cx);
            }
        }))
        .child(thumb)
        .into_any_element()
}

/// Renders the Playlists destination list (ADR 0084 Decision 2): sort,
/// "+", each playlist row, and the inline new-playlist input.
#[expect(
    clippy::too_many_lines,
    reason = "moves the Library sidebar's existing playlist rows, sort, add, and inline input intact (ADR 0084 Decision 2)"
)]
pub(crate) fn render_library_playlists_list(
    sidebar: PlaylistSidebarVm,
    new_playlist_input: &gpui::Entity<gpui_component::input::InputState>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let PlaylistSidebarVm {
        sort_button_id,
        add_button_id,
        new_playlist_input_id,
        new_playlist_add_button_id,
        sort_label,
        add_label,
        new_playlist_add_label,
        creating_playlist,
        rows,
        ..
    } = sidebar;

    let mut items: Vec<AnyElement> = Vec::new();
    items.push(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_end()
            .gap(spacing::XS)
            .px(spacing::MD)
            .pb(spacing::XS)
            .child(
                UiButton::styled(sort_button_id, ControlStyle::ToolbarIcon)
                    .label(sort_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cycle_playlist_sort(cx);
                    })),
            )
            .child(
                UiButton::styled(add_button_id, ControlStyle::ToolbarIcon)
                    .label(add_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_creating_playlist(cx);
                    })),
            )
            .into_any_element(),
    );

    if creating_playlist {
        items.push(
            div()
                .id(new_playlist_input_id)
                .px(spacing::MD)
                .pb(spacing::XS)
                .flex()
                .flex_row()
                .gap(spacing::XS)
                .items_center()
                .child(
                    Input::new(new_playlist_input)
                        .cleanable(false)
                        .scaled(Size::Small, cx)
                        .with_primary_selection(new_playlist_input),
                )
                .child(
                    UiButton::styled(new_playlist_add_button_id, ControlStyle::Primary)
                        .label(new_playlist_add_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.create_playlist(cx);
                        })),
                )
                .into_any_element(),
        );
    }

    for row in rows {
        let PlaylistSidebarRowVm {
            id: playlist_id,
            element_id,
            name,
            track_count_label,
            selected,
        } = row;
        let row_a11y_label = format!("Playlist: {name}");
        items.push(
            ListRow::new(SharedString::from(element_id))
                .a11y_label(ListRowA11yLabel {
                    label: row_a11y_label.into(),
                })
                .selected(selected)
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.open_page(
                        FrameNavigationEntry::PlaylistDetail(playlist_id),
                        NavigationPathStart::CurrentPath,
                        cx,
                    );
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .pl(spacing::MD)
                        .child(
                            Label::new(name)
                                .color(if selected {
                                    SemanticColor::Accent
                                } else {
                                    SemanticColor::Label
                                })
                                .truncated(),
                        )
                        .child(DisclosureSupplementLabel::new(
                            DisclosureSupplementDisplay {
                                label: track_count_label.into(),
                            },
                        )),
                )
                .into_any_element(),
        );
    }

    div()
        .id("music-playlists-destination-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .pt(spacing::MD)
        .pb(spacing::MD)
        .gap(spacing::XXS)
        .children(items)
        .into_any_element()
}
