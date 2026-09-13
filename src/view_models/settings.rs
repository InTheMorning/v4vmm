//! Session-local Settings navigation and existing control contracts (ADR 0069).

#![warn(clippy::pedantic)]

use crate::config::UiScale;
use crate::theme_profile::ThemeProfile;

use super::startup::StartupAvailability;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsGroup {
    #[default]
    General,
    Library,
    Diagnostics,
}

impl SettingsGroup {
    pub(crate) const ALL: [Self; 3] = [Self::General, Self::Library, Self::Diagnostics];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Library => "Library",
            Self::Diagnostics => "Diagnostics",
        }
    }

    pub(crate) const fn contents(self) -> &'static [SettingsContent] {
        use SettingsContent::{
            BackgroundReports, CachedFiles, Endpoint, FlacPath, MusicDirectory, Scale,
            SessionMaintenance, Theme,
        };
        match self {
            Self::General => &[Scale, Theme],
            Self::Library => &[Endpoint, MusicDirectory, FlacPath],
            Self::Diagnostics => &[SessionMaintenance, BackgroundReports, CachedFiles],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsContent {
    Scale,
    Theme,
    Endpoint,
    MusicDirectory,
    FlacPath,
    BackgroundReports,
    CachedFiles,
    SessionMaintenance,
}

impl SettingsContent {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Scale => "UI scale",
            Self::Theme => "Theme",
            Self::Endpoint => "MusicIndex endpoint",
            Self::MusicDirectory => "Music directory",
            Self::FlacPath => "flac binary (optional)",
            Self::BackgroundReports => "Background tools",
            Self::CachedFiles => "Cached files",
            Self::SessionMaintenance => "App session",
        }
    }

    pub(crate) const fn help(self) -> &'static str {
        match self {
            Self::Scale => "Scales the interface. Applies immediately; click Save to persist.",
            Self::Theme => "Applies immediately. Click Save to persist.",
            Self::Endpoint => "Use api.musicindex.org or a full http/https URL.",
            Self::MusicDirectory => "Current session folder. Use Configuration repair below to test and save a different existing folder after ending this session.",
            Self::FlacPath => "Used to silently upgrade WAV downloads to FLAC. Leave blank to resolve flac via $PATH.",
            Self::BackgroundReports | Self::CachedFiles | Self::SessionMaintenance => "",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsAction {
    Open,
    OpenReport,
    SelectGroup(SettingsGroup),
    Save,
    UseDefaults,
    SetScale(UiScale),
    SetTheme(ThemeProfile),
}

/// Only explicit editing actions can produce an effect outside navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsEffect {
    Navigate,
    Save,
    UseDefaults,
    SetScale(UiScale),
    SetTheme(ThemeProfile),
}

#[derive(Clone, Debug)]
pub(crate) struct SettingsActionDisplay {
    pub(crate) action: SettingsAction,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) availability: StartupAvailability,
    pub(crate) selected: bool,
}

#[derive(Debug, Default)]
pub(crate) struct SettingsVm {
    selected: SettingsGroup,
}

impl SettingsVm {
    pub(crate) const TITLE: &'static str = "Settings";
    pub(crate) const SAVE_SCOPE: &'static str =
        "Save applies General and Library controls together. Use Defaults resets and saves these controls; core paths use Configuration repair.";

    pub(crate) const fn shows_configuration_repair(&self) -> bool {
        matches!(
            self.selected,
            SettingsGroup::Library | SettingsGroup::Diagnostics
        )
    }

    pub(crate) const fn selected(&self) -> SettingsGroup {
        self.selected
    }

    pub(crate) const fn editable(&self) -> bool {
        matches!(
            self.selected,
            SettingsGroup::General | SettingsGroup::Library
        )
    }

    pub(crate) fn dispatch(&mut self, action: SettingsAction) -> SettingsEffect {
        match action {
            SettingsAction::Open => SettingsEffect::Navigate,
            SettingsAction::OpenReport => {
                self.selected = SettingsGroup::Diagnostics;
                SettingsEffect::Navigate
            }
            SettingsAction::SelectGroup(group) => {
                self.selected = group;
                SettingsEffect::Navigate
            }
            SettingsAction::Save => SettingsEffect::Save,
            SettingsAction::UseDefaults => SettingsEffect::UseDefaults,
            SettingsAction::SetScale(scale) => SettingsEffect::SetScale(scale),
            SettingsAction::SetTheme(profile) => SettingsEffect::SetTheme(profile),
        }
    }

    pub(crate) fn navigation(&self) -> Vec<SettingsActionDisplay> {
        SettingsGroup::ALL
            .into_iter()
            .map(|group| SettingsActionDisplay {
                action: SettingsAction::SelectGroup(group),
                id: format!("settings-group-{}", group.label()),
                label: group.label().into(),
                a11y_label: format!(
                    "Open {} settings{}",
                    group.label(),
                    if self.selected == group {
                        ", selected"
                    } else {
                        ""
                    }
                ),
                availability: StartupAvailability::Available,
                selected: self.selected == group,
            })
            .collect()
    }

    pub(crate) fn edit_actions(&self, correction_idle: bool) -> Vec<SettingsActionDisplay> {
        if !self.editable() {
            return Vec::new();
        }
        [
            (
                SettingsAction::Save,
                "settings-save",
                "Save",
                "Save General and Library settings",
            ),
            (
                SettingsAction::UseDefaults,
                "settings-default",
                "Use Defaults",
                "Reset and save General and Library defaults",
            ),
        ]
        .into_iter()
        .map(|(action, id, label, a11y_label)| SettingsActionDisplay {
            action,
            id: id.into(),
            label: label.into(),
            a11y_label: a11y_label.into(),
            availability: if correction_idle {
                StartupAvailability::Available
            } else {
                StartupAvailability::Working
            },
            selected: false,
        })
        .collect()
    }

    pub(crate) fn scale_choices(current: UiScale) -> Vec<SettingsActionDisplay> {
        [
            (UiScale::XSmall, "XS", "Extra small"),
            (UiScale::Small, "S", "Small"),
            (UiScale::Medium, "M", "Medium"),
            (UiScale::Large, "L", "Large"),
            (UiScale::XLarge, "XL", "Extra large"),
        ]
        .into_iter()
        .map(|(scale, label, full_label)| SettingsActionDisplay {
            action: SettingsAction::SetScale(scale),
            id: format!("ui-scale-{}", scale.as_str()),
            label: label.into(),
            a11y_label: format!("{full_label} UI scale"),
            availability: StartupAvailability::Available,
            selected: current == scale,
        })
        .collect()
    }

    pub(crate) fn theme_choices(current: ThemeProfile) -> Vec<SettingsActionDisplay> {
        ThemeProfile::USER_SELECTABLE
            .into_iter()
            .map(|profile| SettingsActionDisplay {
                action: SettingsAction::SetTheme(profile),
                id: profile.as_str().into(),
                label: profile.settings_label().into(),
                a11y_label: format!("{} theme profile", profile.settings_label()),
                availability: StartupAvailability::Available,
                selected: current == profile,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0069_group_contract_has_stable_membership_and_accessible_actions() {
        let mut vm = SettingsVm::default();
        let ids = vm
            .navigation()
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        assert_eq!(vm.selected(), SettingsGroup::General);
        for group in SettingsGroup::ALL {
            assert_eq!(
                vm.dispatch(SettingsAction::SelectGroup(group)),
                SettingsEffect::Navigate
            );
            let navigation = vm.navigation();
            assert_eq!(
                navigation.iter().map(|item| &item.id).collect::<Vec<_>>(),
                ids.iter().collect::<Vec<_>>()
            );
            assert_eq!(
                navigation
                    .iter()
                    .map(|item| item.label.as_str())
                    .collect::<Vec<_>>(),
                ["General", "Library", "Diagnostics"]
            );
            assert_eq!(navigation.iter().filter(|item| item.selected).count(), 1);
            for item in navigation {
                assert_eq!(item.availability, StartupAvailability::Available);
                assert!(!item.a11y_label.is_empty());
                assert_eq!(
                    item.selected,
                    item.action == SettingsAction::SelectGroup(group)
                );
            }
        }
        assert_eq!(
            SettingsGroup::General.contents(),
            &[SettingsContent::Scale, SettingsContent::Theme]
        );
        assert_eq!(
            SettingsGroup::Library.contents(),
            &[
                SettingsContent::Endpoint,
                SettingsContent::MusicDirectory,
                SettingsContent::FlacPath
            ]
        );
        assert_eq!(
            SettingsGroup::Diagnostics.contents(),
            &[
                SettingsContent::SessionMaintenance,
                SettingsContent::BackgroundReports,
                SettingsContent::CachedFiles
            ]
        );
    }

    #[test]
    fn adr_0069_navigation_and_reentry_emit_no_edit_effect() {
        let mut vm = SettingsVm::default();
        for group in SettingsGroup::ALL {
            assert_eq!(
                vm.dispatch(SettingsAction::SelectGroup(group)),
                SettingsEffect::Navigate
            );
            for _ in 0..3 {
                assert_eq!(vm.dispatch(SettingsAction::Open), SettingsEffect::Navigate);
                assert_eq!(vm.selected(), group);
            }
        }
        vm.dispatch(SettingsAction::SelectGroup(SettingsGroup::Library));
        assert_eq!(
            vm.dispatch(SettingsAction::OpenReport),
            SettingsEffect::Navigate
        );
        assert_eq!(vm.selected(), SettingsGroup::Diagnostics);
    }

    #[test]
    fn adr_0069_edit_actions_have_one_shared_whole_form_route() {
        let mut vm = SettingsVm::default();
        for group in [SettingsGroup::General, SettingsGroup::Library] {
            vm.dispatch(SettingsAction::SelectGroup(group));
            let actions = vm.edit_actions(true);
            assert!(vm
                .edit_actions(false)
                .iter()
                .all(|action| action.availability == StartupAvailability::Working));
            assert_eq!(actions.len(), 2);
            assert_eq!(vm.dispatch(actions[0].action), SettingsEffect::Save);
            assert_eq!(vm.dispatch(actions[1].action), SettingsEffect::UseDefaults);
            assert!(actions
                .iter()
                .all(|action| action.a11y_label.contains("General and Library")));
            assert_eq!(vm.selected(), group);
        }
        vm.dispatch(SettingsAction::OpenReport);
        assert!(vm.edit_actions(true).is_empty());
        for (choices, expected) in [
            (
                SettingsVm::scale_choices(UiScale::Large),
                SettingsAction::SetScale(UiScale::Large),
            ),
            (
                SettingsVm::theme_choices(ThemeProfile::Light),
                SettingsAction::SetTheme(ThemeProfile::Light),
            ),
        ] {
            assert_eq!(choices.iter().filter(|choice| choice.selected).count(), 1);
            assert_eq!(
                choices
                    .iter()
                    .find(|choice| choice.selected)
                    .unwrap()
                    .action,
                expected
            );
            assert!(choices.iter().all(|choice| !choice.a11y_label.is_empty()));
        }
    }
}
