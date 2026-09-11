//! Shared Settings form, wrapping control rows and bounded content (ADR 0069).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, relative, AnyElement, App, Entity, SharedString, Window};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::Size;

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::layouts as layout;
use crate::ui::primitives::Button;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::settings::{
    SettingsAction, SettingsActionDisplay, SettingsContent, SettingsVm,
};
use crate::view_models::startup::StartupAvailability;

pub(crate) type SettingsCallback = Rc<dyn Fn(SettingsAction, &mut Window, &mut App)>;

pub(crate) fn settings_frame(
    navigation: AnyElement,
    content: Vec<AnyElement>,
    cx: &App,
) -> AnyElement {
    let settings_column_width = layout::scaled_dimension(layout::SETTINGS_COLUMN_WIDTH, cx);
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
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_y_scrollbar()
                .p(Spacing::LG.scaled(cx))
                .child(
                    div()
                        .w(settings_column_width)
                        .max_w(relative(1.0))
                        .flex()
                        .flex_col()
                        .gap(Spacing::LG.scaled(cx))
                        .children(content),
                ),
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
