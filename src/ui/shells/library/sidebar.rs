//! Library sidebar tree surface.
//!
//! Renders the artist, album, and track tree. Selection and expansion state
//! stay on `LibraryApp`; callbacks dispatch back through `cx.listener(...)`.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Image, SharedString};

use crate::library::LibraryApp;
use crate::ui::composites::{
    DisclosureIndicator, DisclosureIndicatorDisplay, DisclosureSupplementDisplay,
    DisclosureSupplementLabel,
};
use crate::ui::shells::library::thumbnail::render_album_thumb;
use crate::ui::style::{color, spacing};
use crate::view_models::library::{
    LibraryAlbumTreeDisplay, LibraryArtistTreeDisplay, LibraryTrackRowVm, LibraryTree,
    LibraryTreeTrackDisplay, LibraryViewModel,
};

#[expect(
    clippy::too_many_lines,
    reason = "lifted legacy sidebar tree renderer stays intact during Task 007 decomposition"
)]
pub(crate) fn render_library_sidebar(
    tree: &LibraryTree,
    expanded_artists: &HashSet<String>,
    expanded_albums: &HashSet<(String, String)>,
    selected_id: Option<i64>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let mut items = Vec::new();
    for artist in &tree.artists {
        let artist_expanded = expanded_artists.contains(&artist.name);
        let LibraryArtistTreeDisplay {
            element_id,
            title,
            disclosure_glyph,
            album_count_label,
        } = artist.tree_display(artist_expanded);
        let artist_name = title.clone();

        items.push(
            div()
                .id(SharedString::from(element_id))
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(spacing::XS)
                .cursor_pointer()
                .hover(|el| el.bg(color::bg_surface_hi()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_artist(&artist_name);
                    this.select_artist(&artist_name, cx);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_baseline()
                        .child(DisclosureIndicator::new(DisclosureIndicatorDisplay {
                            glyph: disclosure_glyph.into(),
                        }))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color::text_primary())
                                .child(SharedString::from(title)),
                        )
                        .child(DisclosureSupplementLabel::new(
                            DisclosureSupplementDisplay {
                                label: album_count_label.into(),
                            },
                        )),
                )
                .into_any_element(),
        );

        if artist_expanded {
            for album in &artist.albums {
                let album_key = (artist.name.clone(), album.name.clone());
                let album_expanded = expanded_albums.contains(&album_key);
                let LibraryAlbumTreeDisplay {
                    element_id,
                    title,
                    disclosure_glyph,
                    track_count_label,
                } = album.tree_display(&artist.name, album_expanded);
                let artist_for_toggle = artist.name.clone();
                let album_for_toggle = album.name.clone();
                let album_for_select = album.clone();
                let thumb_url = album.image_href.clone();
                let thumb_image = thumb_url
                    .as_ref()
                    .and_then(|url| album_thumbs.get(url.as_str()))
                    .and_then(Clone::clone);

                items.push(
                    div()
                        .id(SharedString::from(element_id))
                        .pl(spacing::LG + spacing::XS)
                        .pr(spacing::SM)
                        .py(spacing::XXS)
                        .rounded(spacing::XS)
                        .cursor_pointer()
                        .hover(|el| el.bg(color::bg_surface_hi()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_album(&artist_for_toggle, &album_for_toggle);
                            this.select_album(&album_for_select, cx);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(spacing::XS)
                                .items_center()
                                .child(DisclosureIndicator::new(DisclosureIndicatorDisplay {
                                    glyph: disclosure_glyph.into(),
                                }))
                                .child(hoverable_thumb(
                                    thumb_url.clone(),
                                    thumb_image.clone(),
                                    34.0,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(color::accent())
                                        .child(SharedString::from(title)),
                                )
                                .child(DisclosureSupplementLabel::new(
                                    DisclosureSupplementDisplay {
                                        label: track_count_label.into(),
                                    },
                                )),
                        )
                        .into_any_element(),
                );

                if album_expanded {
                    for track in &album.tracks {
                        let track_clone_b = track.clone();
                        let is_selected = selected_id == Some(track.id);
                        let LibraryTreeTrackDisplay { element_id, title } =
                            LibraryTrackRowVm::new(track, None).tree_display();
                        let track_thumb_image = track
                            .track_image_href
                            .as_ref()
                            .or(track.album_image_href.as_ref())
                            .and_then(|url| album_thumbs.get(url.as_str()))
                            .and_then(Clone::clone);

                        let row = div()
                            .id(SharedString::from(element_id))
                            .pl(spacing::XXL + spacing::MD)
                            .pr(spacing::SM)
                            .py(spacing::XXS)
                            .rounded(spacing::XS)
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(color::bg_selected()))
                            .when(is_selected, |el| {
                                el.border_l_2().border_color(color::accent())
                            })
                            .hover(|el| el.bg(color::bg_surface_hi()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_track(&track_clone_b, cx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(spacing::XS)
                                    .child(render_album_thumb(track_thumb_image.clone(), 24.0, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_xs()
                                            .text_color(if is_selected {
                                                color::accent()
                                            } else {
                                                color::text_primary()
                                            })
                                            .child(SharedString::from(title)),
                                    ),
                            );

                        items.push(row.into_any_element());
                    }
                }
            }
        }
    }
    items
}

fn hoverable_thumb(
    url: Option<String>,
    image: Option<Arc<Image>>,
    size: f32,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let inner = render_album_thumb(image, size, cx);
    let Some(url) = url else {
        return inner;
    };
    let enter_url = url.clone();
    let leave_url = url.clone();
    let display = LibraryViewModel::hover_thumb_display(&url);
    div()
        .id(SharedString::from(display.element_id))
        .on_mouse_move(cx.listener(move |this, _, _, cx| {
            if this.hovered_thumb_url() != Some(enter_url.as_str()) {
                this.set_hovered_thumb(Some(enter_url.clone()), cx);
            }
        }))
        .on_hover(cx.listener(move |this, entered: &bool, _, cx| {
            if !*entered && this.hovered_thumb_url() == Some(leave_url.as_str()) {
                this.set_hovered_thumb(None, cx);
            }
        }))
        .child(inner)
        .into_any_element()
}
