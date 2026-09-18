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
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
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
    pages: [ScrollHandle; 5],
}

impl SettingsScrollHandles {
    pub(crate) fn diagnostic_handle(
        &self,
        page: crate::view_models::settings::DiagnosticPage,
    ) -> &ScrollHandle {
        &self.pages[page as usize]
    }

    pub(crate) fn show_start(&self, group: SettingsGroup) {
        self.handle(group).set_offset(gpui::Point::default());
    }

    pub(crate) const fn handle(&self, group: SettingsGroup) -> &ScrollHandle {
        match group {
            SettingsGroup::General => &self.general,
            SettingsGroup::Library => &self.library,
            SettingsGroup::Diagnostics => &self.diagnostics,
        }
    }
}

pub(crate) fn settings_frame(
    navigation: Vec<AnyElement>,
    content: Vec<AnyElement>,
    scroll_handle: &ScrollHandle,
    workspace: bool,
    cx: &App,
) -> AnyElement {
    div()
        .id("settings")
        .text_size(FontSize::Body.scaled(cx))
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .bg(color(cx, SemanticColor::SystemBackground))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .flex_shrink_0()
                .gap(Spacing::SM.scaled(cx))
                .px(Spacing::MD.scaled(cx))
                .pt(Spacing::MD.scaled(cx))
                .child(
                    div()
                        .child(settings_heading(SettingsVm::TITLE, cx))
                        .debug_selector(|| "settings-title".to_owned()),
                )
                .children(navigation)
                .map(|header| {
                    #[cfg(test)]
                    let header = header.debug_selector(|| "settings-header".to_owned());
                    header
                }),
        )
        .child(if workspace {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_hidden()
                .p(Spacing::MD.scaled(cx))
                .children(content)
                .into_any_element()
        } else {
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
                .vertical_scrollbar(scroll_handle)
                .into_any_element()
        })
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

pub(crate) fn settings_plain_page(
    content: Vec<AnyElement>,
    scroll: &ScrollHandle,
    cx: &App,
) -> AnyElement {
    div()
        .id("diagnostic-files")
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .child(
            page_scroll_content(cx)
                .gap(Spacing::SM.scaled(cx))
                .children(content),
        )
        .vertical_scrollbar(scroll)
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
                .min_w_0()
                .with_primary_selection(input),
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
    use crate::ui::composites::log_frame::LogFrames;
    use crate::ui::composites::maintenance_forms::{database_tools, DatabaseCallback};
    use crate::ui::composites::maintenance_page::{PageCallback, PageMenu, PageNavigation};
    use crate::ui::composites::startup_report::{startup_report, RecoveryNavigation};
    use crate::ui::tokens::ScaleFactor;
    use crate::view_models::maintenance::{MaintenanceView, RecoveryPage};
    use crate::view_models::startup::database::{DatabaseTask, DatabaseVm};
    use crate::view_models::startup::{StartupAction, StartupReportVm};
    use gpui::{
        AppContext, Context, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    };

    struct ShortWindowTest {
        recovery: bool,
        database: DatabaseVm,
        inputs: [Entity<InputState>; 3],
        logs: LogFrames,
        scroll: ScrollHandle,
    }

    impl Render for ShortWindowTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let command: DatabaseCallback = Rc::new(|_, _, _| {});
            let task: PageCallback<DatabaseTask> = Rc::new(|_, _, _| {});
            let page = database_tools(
                &self.database,
                [&self.inputs[0], &self.inputs[1], &self.inputs[2]],
                &command,
                &self.logs,
                PageNavigation {
                    view: self.database.view,
                    select: Rc::new(|_, _, _| {}),
                    scroll: self.scroll.clone(),
                },
                &task,
                cx,
            );
            if self.recovery {
                let command: PageCallback<StartupAction> = Rc::new(|_, _, _| {});
                return startup_report(
                    &StartupReportVm::new(true),
                    &command,
                    None,
                    Some(page),
                    &self.logs,
                    RecoveryNavigation {
                        page: RecoveryPage::Database,
                        select: Rc::new(|_, _, _| {}),
                        startup: PageNavigation {
                            view: MaintenanceView::Instructions,
                            select: Rc::new(|_, _, _| {}),
                            scroll: ScrollHandle::default(),
                        },
                    },
                    cx,
                )
                .into_any_element();
            }
            let mut vm = SettingsVm::default();
            vm.dispatch(SettingsAction::SelectGroup(SettingsGroup::Diagnostics));
            let callback: SettingsCallback = Rc::new(|_, _, _| {});
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(settings_frame(
                    vec![
                        PageMenu::new(
                            "test-settings-menu",
                            SettingsVm::GROUP_MENU,
                            vm.navigation(),
                            callback.clone(),
                        )
                        .into_any_element(),
                        PageMenu::new(
                            "test-diagnostics-menu",
                            SettingsVm::DIAGNOSTIC_MENU,
                            vm.diagnostic_navigation(),
                            callback,
                        )
                        .into_any_element(),
                    ],
                    vec![page],
                    &ScrollHandle::default(),
                    true,
                    cx,
                ))
                .into_any_element()
        }
    }

    /// Situational ADR 0074: complete navigation must leave usable content at short heights.
    #[gpui::test]
    fn adr_0074_short_windows_reserve_space_for_task_content(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        for recovery in [false, true] {
            for scale in [ScaleFactor::Medium, ScaleFactor::XLarge] {
                cx.update(|cx| cx.set_global(scale));
                for width in [438., 1407.] {
                    let (page, cx) = cx.add_window_view(|window, cx| ShortWindowTest {
                        recovery,
                        database: DatabaseVm::new(true),
                        inputs: std::array::from_fn(|_| cx.new(|cx| InputState::new(window, cx))),
                        logs: LogFrames::default(),
                        scroll: ScrollHandle::default(),
                    });
                    cx.simulate_resize(gpui::size(gpui::px(width), gpui::px(382.)));
                    cx.run_until_parked();
                    for task in [DatabaseTask::Check, DatabaseTask::Repair] {
                        for view in [MaintenanceView::Instructions, MaintenanceView::Report] {
                            page.update(cx, |page, cx| {
                                page.database.task = task;
                                page.database.view = view;
                                page.database.report = "Database result\n".repeat(100);
                                cx.notify();
                            });
                            cx.update(|window, cx| {
                                let _ = window.draw(cx);
                            });
                            let main = cx.debug_bounds("maintenance-content").unwrap();
                            let actions = cx.debug_bounds("maintenance-actions").unwrap();
                            let header = cx.debug_bounds("settings-header");
                            let toolbar = cx.debug_bounds("maintenance-toolbar");
                            let container = cx.debug_bounds("maintenance-main").unwrap();
                            let toggle = cx.debug_bounds("maintenance-view-toggle").unwrap();
                            assert!(
                                toggle.left() >= container.left()
                                    && toggle.right() <= container.right()
                            );
                            assert!(
                                toggle.top() >= container.top() && toggle.bottom() <= main.top()
                            );
                            if !recovery {
                                let title = cx
                                    .debug_bounds("settings-title")
                                    .expect("Diagnostics retains Settings title");
                                let header = header.unwrap();
                                assert!(
                                    title.top() >= header.top()
                                        && title.bottom() <= header.bottom()
                                );
                            }
                            let minimum = if width > 760. { 200. } else { 110. };
                            assert!(main.size.height >= gpui::px(minimum),
                                "recovery={recovery} scale={scale:?} width={width} task={task:?} view={view:?}: {main:?} actions={actions:?} header={header:?} toolbar={toolbar:?}");
                            assert!(main.bottom() <= gpui::px(382.));
                            assert!(main.right() <= gpui::px(width));
                            if width < 760. {
                                assert!(actions.bottom() <= main.top());
                                assert!(actions.size.height < main.size.height);
                            } else {
                                assert!(actions.right() <= main.left());
                            }
                            if view == MaintenanceView::Instructions {
                                cx.simulate_event(ScrollWheelEvent {
                                    position: main.center(),
                                    delta: ScrollDelta::Pixels(gpui::point(
                                        gpui::px(0.),
                                        gpui::px(-3000.),
                                    )),
                                    modifiers: Default::default(),
                                    touch_phase: TouchPhase::Moved,
                                });
                                cx.update(|window, cx| {
                                    let _ = window.draw(cx);
                                });
                                assert_eq!(
                                    cx.debug_bounds("maintenance-actions").unwrap(),
                                    actions
                                );
                            }
                        }
                    }
                }
            }
        }
    }

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
