//! `MusicBrainz` lookup panel composite.
//!
//! Owns the shared lookup presentation: release picker, selected recording
//! summary, empty-state copy, and artwork placement. Screens provide the
//! pre-resolved image and selection callback.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, FontWeight, Image, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable, Size};

use crate::ui::layouts as layout;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::musicbrainz_panel::MusicBrainzPanelVm;

use super::{EntityKind, Thumbnail, ThumbnailSize};

type SelectHandler = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
#[must_use]
pub struct MusicBrainzPanel {
    vm: MusicBrainzPanelVm,
    image: Option<Arc<Image>>,
    on_select: Option<SelectHandler>,
    appearance: Option<Appearance>,
}

impl MusicBrainzPanel {
    pub fn new(vm: MusicBrainzPanelVm) -> Self {
        Self {
            vm,
            image: None,
            on_select: None,
            appearance: None,
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) + 'static,
    {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for MusicBrainzPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_color = resolve_color(cx, SemanticColor::Label, self.appearance);
        let subtitle_color = resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance);

        match (self.vm.candidate_title(), self.vm.candidate_subtitle()) {
            (Some(title), Some(subtitle)) => div()
                .flex()
                .flex_row()
                .items_start()
                .gap(Spacing::LG.scaled(cx))
                .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(self.image))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(title_bar(&self.vm, self.on_select, self.appearance, cx))
                        .child(
                            div()
                                .text_size(FontSize::Title3.scaled(cx))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(title_color)
                                .child(SharedString::from(title.to_string())),
                        )
                        .child(
                            div()
                                .text_color(subtitle_color)
                                .text_size(FontSize::Micro.scaled(cx))
                                .line_clamp(2)
                                .child(SharedString::from(subtitle.to_string())),
                        ),
                )
                .into_any_element(),
            _ => div()
                .flex()
                .flex_col()
                .gap(Spacing::SM.scaled(cx))
                .child(title_bar(&self.vm, self.on_select, self.appearance, cx))
                .child(
                    div()
                        .text_size(FontSize::Micro.scaled(cx))
                        .text_color(subtitle_color)
                        .child("No MusicBrainz recording match found"),
                )
                .into_any_element(),
        }
    }
}

fn title_bar(
    vm: &MusicBrainzPanelVm,
    on_select: Option<SelectHandler>,
    _appearance: Option<Appearance>,
    cx: &mut App,
) -> AnyElement {
    let badge_fill = EntityKind::Track.fill_color(cx);
    let badge_text = EntityKind::Track.on_fill_color(cx);
    // CONTROL-COMPAT(reason): native Button does not yet expose dropdown_menu, full-width alignment, and custom badge fill styling.
    let trigger = Button::new("musicbrainz-release-picker")
        .label(SharedString::from(vm.trigger_label().to_string()))
        .scaled(Size::XSmall, cx)
        .compact()
        .ghost()
        .w_full()
        .justify_start()
        .bg(badge_fill)
        .text_color(badge_text)
        .text_size(FontSize::Micro.scaled(cx))
        .font_weight(FontWeight::BOLD)
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XXS.scaled(cx))
        .border_1()
        .border_color(badge_fill)
        .rounded(Radius::SM.scaled(cx))
        .mb(Spacing::SM.scaled(cx));

    if !vm.has_candidates() {
        return trigger.disabled(true).into_any_element();
    }

    let options = vm.options().to_vec();
    trigger
        .dropdown_menu(move |menu, _window, _cx| {
            options.iter().enumerate().fold(
                menu.min_w(layout::MENU_MIN_WIDTH)
                    .max_w(layout::MENU_MAX_WIDTH)
                    .scrollable(true),
                |menu, (idx, option)| {
                    let on_select = on_select.clone();
                    menu.item(
                        PopupMenuItem::new(SharedString::from(option.label.clone()))
                            .checked(option.selected)
                            .on_click(move |_, window, cx| {
                                if let Some(on_select) = &on_select {
                                    on_select(idx, window, cx);
                                }
                            }),
                    )
                },
            )
        })
        .into_any_element()
}
