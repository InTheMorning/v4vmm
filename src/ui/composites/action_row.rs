//! Inspector action-row composite.
//!
//! Screens keep command wiring and target lookup. This composite owns the
//! shared presentation contract for inspector actions: a compact vertical
//! stack, optional wrapped control groups, and tokenized neutral/danger
//! status messages.

#![warn(clippy::pedantic)]

use gpui::{
    div, AnyElement, App, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::layouts as layout;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionRowMessageTone {
    Neutral,
    Danger,
}

#[derive(Clone, Debug)]
#[must_use]
pub struct ActionRowMessage {
    text: SharedString,
    tone: ActionRowMessageTone,
    max_width: Pixels,
}

impl ActionRowMessage {
    pub fn neutral(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: ActionRowMessageTone::Neutral,
            max_width: layout::STATUS_MESSAGE_WIDTH,
        }
    }

    pub fn danger(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: ActionRowMessageTone::Danger,
            max_width: layout::ACTION_MESSAGE_WIDTH,
        }
    }

    pub const fn max_width(mut self, max_width: Pixels) -> Self {
        self.max_width = max_width;
        self
    }
}

enum ActionRowItem {
    Control(AnyElement),
    ControlGroup(Vec<AnyElement>),
    Message(ActionRowMessage),
}

#[derive(IntoElement)]
#[must_use]
pub struct ActionRow {
    items: Vec<ActionRowItem>,
    appearance: Option<Appearance>,
}

impl ActionRow {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            appearance: None,
        }
    }

    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.items
            .push(ActionRowItem::Control(control.into_any_element()));
        self
    }

    pub fn control_group(mut self, controls: Vec<AnyElement>) -> Self {
        self.items.push(ActionRowItem::ControlGroup(controls));
        self
    }

    pub fn message(mut self, message: ActionRowMessage) -> Self {
        self.items.push(ActionRowItem::Message(message));
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl Default for ActionRow {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for ActionRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut row = div()
            .flex()
            .flex_col()
            .items_start()
            .gap(Spacing::XS.scaled(cx));

        for item in self.items {
            row = match item {
                ActionRowItem::Control(control) => row.child(control),
                ActionRowItem::ControlGroup(controls) => {
                    row.child(render_control_group(controls, cx))
                }
                ActionRowItem::Message(message) => {
                    row.child(render_message(message, self.appearance, cx))
                }
            };
        }

        row
    }
}

fn render_control_group(controls: Vec<AnyElement>, cx: &mut App) -> impl IntoElement {
    let mut group = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .flex_wrap();

    for control in controls {
        group = group.child(control);
    }

    group
}

fn render_message(
    message: ActionRowMessage,
    appearance: Option<Appearance>,
    cx: &mut App,
) -> impl IntoElement {
    let color = match message.tone {
        ActionRowMessageTone::Neutral => SemanticColor::SecondaryLabel,
        ActionRowMessageTone::Danger => SemanticColor::DangerLabel,
    };

    div()
        .max_w(message.max_width)
        .text_size(FontSize::Micro.scaled(cx))
        .text_color(resolve_color(cx, color, appearance))
        .child(message.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_defaults_to_expected_widths_and_tones() {
        let neutral = ActionRowMessage::neutral("Saved");
        assert_eq!(neutral.tone, ActionRowMessageTone::Neutral);
        assert_eq!(neutral.max_width, layout::STATUS_MESSAGE_WIDTH);

        let danger = ActionRowMessage::danger("Error");
        assert_eq!(danger.tone, ActionRowMessageTone::Danger);
        assert_eq!(danger.max_width, layout::ACTION_MESSAGE_WIDTH);
    }

    #[test]
    fn message_width_can_be_overridden() {
        let message =
            ActionRowMessage::danger("Duplicate").max_width(layout::CONFLICT_MESSAGE_WIDTH);
        assert_eq!(message.max_width, layout::CONFLICT_MESSAGE_WIDTH);
    }
}
