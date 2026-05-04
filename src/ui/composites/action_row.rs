//! Inspector action-row composite.
//!
//! Screens keep command wiring and target lookup. This composite owns the
//! shared presentation contract for inspector actions: a compact vertical
//! stack, optional wrapped control groups, and tokenized neutral/danger
//! status messages.
//!
//! Accessibility note (ADR 0038 task 005): [`ActionRowDisplay`] carries a
//! VM-sourced group label. GPUI 0.2.x does not expose an accessibility group
//! sink for a plain `div`, so the value is retained as a contract field until
//! the framework can consume it.

#![warn(clippy::pedantic)]

use gpui::{
    div, AnyElement, App, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::layouts as layout;
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::{
    ActionStatusMessageDisplay, ActionStatusMessageTone, ActionStatusMessageWidth,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRowMessageDisplay {
    pub text: SharedString,
    pub tone: ActionRowMessageTone,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRowDisplay {
    pub a11y_label: SharedString,
}

impl ActionRowMessage {
    pub fn new(display: ActionRowMessageDisplay) -> Self {
        let max_width = match display.tone {
            ActionRowMessageTone::Neutral => layout::STATUS_MESSAGE_WIDTH,
            ActionRowMessageTone::Danger => layout::ACTION_MESSAGE_WIDTH,
        };
        Self {
            text: display.text,
            tone: display.tone,
            max_width,
        }
    }

    pub(crate) fn from_status_display(display: ActionStatusMessageDisplay) -> Self {
        let tone = match display.tone {
            ActionStatusMessageTone::Neutral => ActionRowMessageTone::Neutral,
            ActionStatusMessageTone::Danger => ActionRowMessageTone::Danger,
        };
        let max_width = match display.width {
            ActionStatusMessageWidth::Status => layout::STATUS_MESSAGE_WIDTH,
            ActionStatusMessageWidth::Action => layout::ACTION_MESSAGE_WIDTH,
            ActionStatusMessageWidth::Conflict => layout::CONFLICT_MESSAGE_WIDTH,
        };
        Self {
            text: SharedString::from(display.text),
            tone,
            max_width,
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
    a11y_label: SharedString,
    items: Vec<ActionRowItem>,
    appearance: Option<Appearance>,
}

impl ActionRow {
    pub fn new(display: ActionRowDisplay) -> Self {
        Self {
            a11y_label: display.a11y_label,
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
        Self::new(ActionRowDisplay {
            a11y_label: SharedString::from("Actions"),
        })
    }
}

impl RenderOnce for ActionRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        std::mem::drop(self.a11y_label);
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
    fn action_row_carries_group_accessibility_label() {
        let row = ActionRow::new(ActionRowDisplay {
            a11y_label: SharedString::from("Track actions"),
        });

        assert_eq!(row.a11y_label, SharedString::from("Track actions"));
    }

    #[test]
    fn message_defaults_to_expected_widths_and_tones() {
        let neutral = ActionRowMessage::new(ActionRowMessageDisplay {
            text: SharedString::from("Saved"),
            tone: ActionRowMessageTone::Neutral,
        });
        assert_eq!(neutral.tone, ActionRowMessageTone::Neutral);
        assert_eq!(neutral.max_width, layout::STATUS_MESSAGE_WIDTH);

        let danger = ActionRowMessage::new(ActionRowMessageDisplay {
            text: SharedString::from("Error"),
            tone: ActionRowMessageTone::Danger,
        });
        assert_eq!(danger.tone, ActionRowMessageTone::Danger);
        assert_eq!(danger.max_width, layout::ACTION_MESSAGE_WIDTH);
    }

    #[test]
    fn message_width_can_be_overridden() {
        let message = ActionRowMessage::new(ActionRowMessageDisplay {
            text: SharedString::from("Duplicate"),
            tone: ActionRowMessageTone::Danger,
        })
        .max_width(layout::CONFLICT_MESSAGE_WIDTH);
        assert_eq!(message.max_width, layout::CONFLICT_MESSAGE_WIDTH);
    }
}
