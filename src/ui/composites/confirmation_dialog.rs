//! Confirmation dialog composite for focused, content-affecting choices.
//!
//! The composite owns the app's dialog body contract: token-driven text,
//! shared button primitives, explicit Cancel affordance, and no domain logic.
//! Screens provide display-ready strings and callbacks.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, App, FontWeight, SharedString, Window};
use gpui_component::{dialog::Dialog, WindowExt};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, ButtonSize, Label, LabelVariant};
use crate::ui::tokens::{SemanticColor, Size, Spacing};

type DialogActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmationDialogDisplay {
    pub title: SharedString,
    pub message: SharedString,
    pub cancel_button_id: SharedString,
    pub cancel_label: SharedString,
    pub cancel_a11y_label: SharedString,
    pub confirm_button_id: SharedString,
    pub confirm_label: SharedString,
    pub confirm_a11y_label: SharedString,
    pub destructive: bool,
    /// The items that the choice affects. The dialog lists them in a
    /// scrolling column below the message. An empty list shows no column.
    pub items: Vec<ConfirmationDialogItem>,
}

/// One affected item of a confirmation. The screen supplies display-ready
/// text. The mark text states its meaning, so its color is never the only
/// signal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmationDialogItem {
    pub title: SharedString,
    pub subtitle: Option<SharedString>,
    pub mark: Option<(SharedString, SemanticColor)>,
    pub lines: Vec<SharedString>,
}

#[derive(Clone)]
pub struct ConfirmationDialogHandlers {
    on_cancel: DialogActionHandler,
    on_confirm: DialogActionHandler,
}

impl ConfirmationDialogHandlers {
    pub fn new(
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
        on_confirm: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            on_cancel: Rc::new(on_cancel),
            on_confirm: Rc::new(on_confirm),
        }
    }
}

#[must_use]
pub fn confirmation_dialog(
    dialog: Dialog,
    display: ConfirmationDialogDisplay,
    handlers: ConfirmationDialogHandlers,
    cx: &App,
) -> Dialog {
    let ConfirmationDialogHandlers {
        on_cancel,
        on_confirm,
    } = handlers;
    let escape_cancel = on_cancel.clone();
    let button_cancel = on_cancel;
    let button_confirm = on_confirm;
    let confirm_style = if display.destructive {
        ControlStyle::Destructive
    } else {
        ControlStyle::Primary
    };

    dialog
        .title(display.title)
        .overlay_closable(false)
        .close_button(false)
        .on_cancel(move |_, window, cx| {
            escape_cancel(window, cx);
            true
        })
        .child(
            div()
                .w(Size::ColumnRegular.scaled(cx))
                .max_w(Size::ColumnTall.scaled(cx))
                .flex()
                .flex_col()
                .gap(Spacing::MD.scaled(cx))
                .child(
                    Label::new(display.message)
                        .variant(LabelVariant::Body)
                        .color(SemanticColor::SecondaryLabel),
                )
                .when(!display.items.is_empty(), |el| {
                    el.child(confirmation_items(display.items, cx))
                })
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap(Spacing::SM.scaled(cx))
                        .child(
                            Button::styled(display.cancel_button_id, ControlStyle::Secondary)
                                .size(ButtonSize::Md)
                                .label(display.cancel_label)
                                .a11y_label(display.cancel_a11y_label)
                                .on_click(move |_, window, cx| {
                                    button_cancel(window, cx);
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::styled(display.confirm_button_id, confirm_style)
                                .size(ButtonSize::Md)
                                .label(display.confirm_label)
                                .a11y_label(display.confirm_a11y_label)
                                .on_click(move |_, window, cx| {
                                    button_confirm(window, cx);
                                    window.close_dialog(cx);
                                }),
                        ),
                ),
        )
}

/// The scrolling item column. Stacked text uses `overflow_hidden()` and no
/// truncation (ADR 0063).
///
/// Each item carries `flex_shrink_0()`.
///
/// A flex child with `overflow_hidden()` and no `flex_shrink_0()` has a
/// minimum height of zero. When the items are taller than `max_h`, the
/// layout shrinks each item to zero height. `flex_shrink_0()` keeps each
/// item at its own height. The column then scrolls to show the items that
/// do not fit (ADR 0076 Decision 8).
fn confirmation_items(items: Vec<ConfirmationDialogItem>, cx: &App) -> impl IntoElement {
    div()
        .id("confirmation-dialog-items")
        .debug_selector(|| "confirmation-dialog-items".to_owned())
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .max_h(Size::ColumnRegular.scaled(cx))
        .overflow_y_scroll()
        .children(items.into_iter().enumerate().map(|(index, item)| {
            div()
                .debug_selector(move || format!("confirmation-dialog-item-{index}"))
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap(Spacing::XXS.scaled(cx))
                .min_w_0()
                .overflow_hidden()
                .child(
                    Label::new(item.title)
                        .variant(LabelVariant::Body)
                        .weight(FontWeight::SEMIBOLD),
                )
                .when_some(item.subtitle, |el, subtitle| {
                    el.child(
                        Label::new(subtitle)
                            .variant(LabelVariant::Caption)
                            .color(SemanticColor::SecondaryLabel),
                    )
                })
                .when_some(item.mark, |el, (mark, color)| {
                    el.child(
                        Label::new(mark)
                            .variant(LabelVariant::Caption)
                            .weight(FontWeight::SEMIBOLD)
                            .color(color),
                    )
                })
                .children(item.lines.into_iter().map(|line| {
                    Label::new(line)
                        .variant(LabelVariant::Caption)
                        .color(SemanticColor::TertiaryLabel)
                }))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_display_keeps_cancel_and_confirm_identity() {
        let display = ConfirmationDialogDisplay {
            title: SharedString::from("Remove Track from Library?"),
            message: SharedString::from("This track is in a playlist."),
            cancel_button_id: SharedString::from("cancel"),
            cancel_label: SharedString::from("Cancel"),
            cancel_a11y_label: SharedString::from("Cancel removing track from library"),
            confirm_button_id: SharedString::from("remove"),
            confirm_label: SharedString::from("Remove"),
            confirm_a11y_label: SharedString::from("Remove track from library"),
            destructive: true,
            items: Vec::new(),
        };

        assert_eq!(display.cancel_label, "Cancel");
        assert_eq!(
            display.cancel_a11y_label,
            SharedString::from("Cancel removing track from library")
        );
        assert_eq!(display.confirm_label, "Remove");
        assert_eq!(
            display.confirm_a11y_label,
            SharedString::from("Remove track from library")
        );
        assert!(display.destructive);
    }

    // -------------------------------------------------------------------
    // ADR 0076 task 008: the item column scrolls instead of shrinking each
    // item to zero height.
    // -------------------------------------------------------------------

    use gpui::{px, AppContext, Context, Render, TestAppContext};
    use gpui_component::Root;

    fn sample_items(count: usize) -> Vec<ConfirmationDialogItem> {
        (0..count)
            .map(|index| ConfirmationDialogItem {
                title: SharedString::from(format!("Track {index}.mp3")),
                subtitle: Some(SharedString::from("Example Album")),
                mark: Some((
                    SharedString::from("Title, Artist"),
                    SemanticColor::SuccessLabel,
                )),
                lines: vec![
                    SharedString::from("TIT2: title"),
                    SharedString::from("TPE1: artist"),
                ],
            })
            .collect()
    }

    fn empty_display() -> ConfirmationDialogDisplay {
        ConfirmationDialogDisplay {
            title: SharedString::from("Remove Track from Library?"),
            message: SharedString::from("This track is in a playlist."),
            cancel_button_id: SharedString::from("cancel"),
            cancel_label: SharedString::from("Cancel"),
            cancel_a11y_label: SharedString::from("Cancel removing track from library"),
            confirm_button_id: SharedString::from("remove"),
            confirm_label: SharedString::from("Remove"),
            confirm_a11y_label: SharedString::from("Remove track from library"),
            destructive: true,
            items: Vec::new(),
        }
    }

    /// Leaks a selector name so a name built at test time satisfies
    /// `VisualTestContext::debug_bounds`'s `'static` key. The test process
    /// ends after the test, so the leak carries no lasting cost.
    fn leaked_selector(name: String) -> &'static str {
        Box::leak(name.into_boxed_str())
    }

    struct ConfirmationItemsTest {
        items: Vec<ConfirmationDialogItem>,
    }

    impl Render for ConfirmationItemsTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(confirmation_items(self.items.clone(), cx))
        }
    }

    /// R76-8-01 and R76-8-02 (ADR 0076 Decision 8).
    ///
    /// Each item keeps its height when 40 items are taller than the
    /// scrolling column. The first item's height stays the same as in a
    /// one-item dialog. The column does not grow past its `max_h`.
    #[gpui::test]
    fn adr_0076_confirmation_list_keeps_item_height_with_many_items(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (view, cx) = cx.add_window_view(|_, _| ConfirmationItemsTest {
            items: sample_items(1),
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let one_item_height = cx
            .debug_bounds(leaked_selector("confirmation-dialog-item-0".to_owned()))
            .expect("the single item renders")
            .size
            .height;

        view.update(cx, |view, cx| {
            view.items = sample_items(40);
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let mut first_item_height = None;
        for index in 0..40 {
            let selector = leaked_selector(format!("confirmation-dialog-item-{index}"));
            let bounds = cx
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("item {index} did not render"));
            assert!(
                bounds.size.height > px(0.0),
                "item {index} has zero height with 40 items"
            );
            if index == 0 {
                first_item_height = Some(bounds.size.height);
            }
        }
        assert_eq!(
            first_item_height,
            Some(one_item_height),
            "the first item's height changed between one item and 40 items"
        );

        let max_height = cx.update(|_, cx| Size::ColumnRegular.scaled(cx));
        let column = cx
            .debug_bounds(leaked_selector("confirmation-dialog-items".to_owned()))
            .expect("the item column renders");
        assert!(
            column.size.height <= max_height,
            "the item column grew past its max_h: {:?} > {max_height:?}",
            column.size.height
        );
    }

    struct ConfirmationDialogTest {
        display: ConfirmationDialogDisplay,
    }

    impl Render for ConfirmationDialogTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let handlers = ConfirmationDialogHandlers::new(|_, _| {}, |_, _| {});
            div().size_full().child(confirmation_dialog(
                Dialog::new(cx),
                self.display.clone(),
                handlers,
                cx,
            ))
        }
    }

    /// R76-8-03: a confirmation with no item shows no item column. The fix
    /// for many items must not change this present behavior.
    #[gpui::test]
    fn adr_0076_confirmation_list_with_no_item_shows_no_column(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (_, cx) = cx.add_window_view(|window, cx| {
            let inner = cx.new(|_| ConfirmationDialogTest {
                display: empty_display(),
            });
            Root::new(inner, window, cx)
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert!(
            cx.debug_bounds(leaked_selector("confirmation-dialog-items".to_owned()))
                .is_none(),
            "an empty confirmation rendered an item column"
        );
    }
}
