//! The shared download-state icon of a release row (ADR 0083 Decision 6,
//! amended 2026-10-22; ADR 0084 task 033).
//!
//! The Releases list, a singles group row, a group disclosure row, the
//! feed page, the name page, the search list and the saved-search page
//! all show the download state of a release through this one composite,
//! in place of the state word each used to show on its own. The view
//! model gives the typed state, the count text, the tooltip and the
//! accessibility label; this composite only maps the typed state to a
//! named icon and a color token.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, App, Div, ElementId, Stateful};

use crate::ui::icons::{Icon, IconName};
use crate::ui::primitives::{Label, Tooltip};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::release_download_state::{ReleaseDownloadState, ReleaseDownloadStateVm};

/// The icon and the color role of a release download state (ADR 0083
/// Decision 6, amended 2026-10-22). `Partial` reads the same quiet icon
/// as `NotDownloaded`: its count text, not a different icon, tells the
/// two apart. Only `Downloading` turns.
const fn icon_role_turning(state: ReleaseDownloadState) -> (IconName, SemanticColor, bool) {
    match state {
        ReleaseDownloadState::NotDownloaded | ReleaseDownloadState::Partial { .. } => {
            (IconName::CloudDownload, SemanticColor::TertiaryLabel, false)
        }
        ReleaseDownloadState::Queued => (IconName::Queued, SemanticColor::SecondaryLabel, false),
        ReleaseDownloadState::Downloading => (IconName::InProgress, SemanticColor::Accent, true),
        ReleaseDownloadState::Downloaded => {
            (IconName::CloudCheck, SemanticColor::SuccessLabel, false)
        }
    }
}

/// Renders the download-state icon of a release row, with its count text
/// when the state carries one (for example "3 of 9"). `id` must be
/// unique among the row's own children. The icon is a quiet mark, never a
/// button: no release row offers an inline download click today (ADR
/// 0084 task 033, Flag), so every state renders the same way.
#[must_use]
pub(crate) fn render_release_download_state(
    vm: &ReleaseDownloadStateVm,
    id: impl Into<ElementId>,
    cx: &App,
) -> AnyElement {
    let (icon_name, role, turning) = icon_role_turning(vm.state);
    let tooltip = Tooltip::new(vm.tooltip);
    let mut cell: Stateful<Div> = div()
        .id(id.into())
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XXS.scaled(cx))
        .tooltip(move |window, cx| tooltip.build(window, cx))
        .aria_label(vm.a11y_label.clone());
    cell = cell.child(Icon::new(icon_name).color(color(cx, role)).turning(turning));
    if let Some(count) = &vm.count_text {
        cell = cell.child(
            Label::new(count.clone())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    cell.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0083 Decision 6, amended 2026-10-22: `Downloading` is the one
    /// state that turns, and it is the one state in the accent color.
    /// `NotDownloaded` and `Partial` read the same icon.
    #[test]
    fn icon_role_turning_maps_each_state() {
        assert_eq!(
            icon_role_turning(ReleaseDownloadState::NotDownloaded),
            (IconName::CloudDownload, SemanticColor::TertiaryLabel, false)
        );
        assert_eq!(
            icon_role_turning(ReleaseDownloadState::Partial {
                downloaded: 3,
                total: 7
            }),
            (IconName::CloudDownload, SemanticColor::TertiaryLabel, false)
        );
        assert_eq!(
            icon_role_turning(ReleaseDownloadState::Queued),
            (IconName::Queued, SemanticColor::SecondaryLabel, false)
        );
        assert_eq!(
            icon_role_turning(ReleaseDownloadState::Downloading),
            (IconName::InProgress, SemanticColor::Accent, true)
        );
        assert_eq!(
            icon_role_turning(ReleaseDownloadState::Downloaded),
            (IconName::CloudCheck, SemanticColor::SuccessLabel, false)
        );
    }
}
