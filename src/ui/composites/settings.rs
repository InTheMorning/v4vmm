//! Shared Settings form, wrapping control rows and bounded content (ADR 0069).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, Entity, ScrollHandle, SharedString, Window};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::Size;

use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::Button;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::settings::{
    SettingsAction, SettingsActionDisplay, SettingsContent, SettingsGroup, SettingsVm,
};
use crate::view_models::startup::StartupAvailability;

pub(crate) type SettingsCallback = Rc<dyn Fn(SettingsAction, &mut Window, &mut App)>;

/// The app root retains each group's page offset while that group is hidden.
#[derive(Default)]
pub(crate) struct SettingsScrollHandles {
    general: ScrollHandle,
    library: ScrollHandle,
    diagnostics: ScrollHandle,
}

impl SettingsScrollHandles {
    pub(crate) const fn handle(&self, group: SettingsGroup) -> &ScrollHandle {
        match group {
            SettingsGroup::General => &self.general,
            SettingsGroup::Library => &self.library,
            SettingsGroup::Diagnostics => &self.diagnostics,
        }
    }
}

pub(crate) fn settings_frame(
    navigation: AnyElement,
    content: Vec<AnyElement>,
    scroll_handle: &ScrollHandle,
    cx: &App,
) -> AnyElement {
    div()
        .id("settings")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .bg(color(cx, SemanticColor::SystemBackground))
        .child(
            div()
                .flex_shrink_0()
                .px(Spacing::LG.scaled(cx))
                .pt(Spacing::MD.scaled(cx))
                .child(settings_heading(SettingsVm::TITLE, cx))
                .child(navigation),
        )
        .child(
            div()
                .id("settings-scroll")
                .relative()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(
                    div()
                        .id("settings-page")
                        .size_full()
                        .min_w_0()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .track_scroll(scroll_handle)
                        .p(Spacing::LG.scaled(cx))
                        .child(
                            page_scroll_content(cx)
                                .gap(Spacing::LG.scaled(cx))
                                .children(content),
                        ),
                )
                .vertical_scrollbar(scroll_handle),
        )
        .into_any_element()
}

pub(crate) fn settings_actions(
    actions: Vec<SettingsActionDisplay>,
    callback: &SettingsCallback,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .children(actions.into_iter().map(|display| {
            let callback = Rc::clone(callback);
            let action = display.action;
            let style = if display.selected || action == SettingsAction::Save {
                ControlStyle::Primary
            } else {
                ControlStyle::Ghost
            };
            let mut button = Button::styled(SharedString::from(display.id), style)
                .label(display.label)
                .a11y_label(display.a11y_label)
                .disabled(display.availability != StartupAvailability::Available)
                .on_activate(move |window, cx| callback(action, window, cx));
            if display.selected {
                button = button.leading_icon(IconName::Check);
            }
            button
        }))
        .into_any_element()
}

pub(crate) fn settings_heading(label: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .min_w_0()
        .text_size(FontSize::Title2.scaled(cx))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(label.into())
        .into_any_element()
}

pub(crate) fn settings_message(message: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .min_w_0()
        .text_size(FontSize::Caption.scaled(cx))
        .text_color(color(cx, SemanticColor::SecondaryLabel))
        .child(message.into())
        .into_any_element()
}

pub(crate) fn settings_field(field: SettingsContent, input: AnyElement, cx: &App) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(field.label()),
        )
        .child(input)
        .child(settings_message(field.help(), cx))
        .into_any_element()
}

pub(crate) fn settings_text_input(input: &Entity<InputState>, cx: &App) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_row()
        .child(
            Input::new(input)
                .cleanable(true)
                .scaled(Size::Small, cx)
                .flex_1()
                .min_w_0(),
        )
        .into_any_element()
}

pub(crate) fn settings_cached_row(title: String, action: Button, cx: &App) -> AnyElement {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .pl(Spacing::MD.scaled(cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(FontSize::Caption.scaled(cx))
                .child(title),
        )
        .child(action)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0069_settings_pages_retain_independent_scroll_offsets() {
        let pages = SettingsScrollHandles::default();
        for group in SettingsGroup::ALL {
            assert_eq!(pages.handle(group).offset(), gpui::Point::default());
        }
        // Use the same shared handles as the viewport, without creating a window.
        {
            let visible = pages.handle(SettingsGroup::Diagnostics).clone();
            visible.set_offset(gpui::point(gpui::px(0.0), gpui::px(-480.0)));
        }
        {
            let visible = pages.handle(SettingsGroup::Library).clone();
            visible.set_offset(gpui::point(gpui::px(0.0), gpui::px(-120.0)));
        }
        assert_eq!(
            pages.handle(SettingsGroup::General).offset().y,
            gpui::px(0.0)
        );
        assert_eq!(
            pages.handle(SettingsGroup::Diagnostics).offset().y,
            gpui::px(-480.0)
        );
        assert_eq!(
            pages.handle(SettingsGroup::Library).offset().y,
            gpui::px(-120.0)
        );
        pages
            .handle(SettingsGroup::Library)
            .set_offset(gpui::Point::default());
        assert_eq!(
            pages.handle(SettingsGroup::Diagnostics).offset().y,
            gpui::px(-480.0)
        );
    }
}
