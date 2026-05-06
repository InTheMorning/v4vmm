//! `MusicBrainz` lookup panel view-model.
//!
//! Pure projection of lookup results into strings and selection state used by
//! the shared `MusicBrainz` panel composite. Screens keep image resolution and
//! selection callbacks.

#![warn(clippy::pedantic)]

use crate::metadata::{
    musicbrainz_release_option_label, musicbrainz_release_summary, musicbrainz_subtitle,
    MusicBrainzLookupResult,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct MusicBrainzPanelVm {
    trigger_label: String,
    candidate_title: Option<String>,
    candidate_subtitle: Option<String>,
    selected_index: Option<usize>,
    options: Vec<MusicBrainzCandidateOptionVm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MusicBrainzCandidateOptionVm {
    pub label: String,
    pub selected: bool,
}

impl MusicBrainzPanelVm {
    pub fn new(result: &MusicBrainzLookupResult, requested_index: usize) -> Self {
        let selected = selected_candidate_index(result, requested_index);
        let selected_candidate = selected.and_then(|idx| result.lookup.candidates.get(idx));
        let trigger_value = selected_candidate.map_or_else(
            || "No MusicBrainz release".to_string(),
            musicbrainz_release_summary,
        );
        let options = result
            .lookup
            .candidates
            .iter()
            .enumerate()
            .map(|(idx, candidate)| MusicBrainzCandidateOptionVm {
                label: musicbrainz_release_option_label(candidate),
                selected: selected == Some(idx),
            })
            .collect();

        Self {
            trigger_label: format!("MusicBrainz: {trigger_value}"),
            candidate_title: selected_candidate.map(|candidate| candidate.title.clone()),
            candidate_subtitle: selected_candidate
                .map(|candidate| musicbrainz_subtitle(requested_index, result, candidate)),
            selected_index: selected,
            options,
        }
    }

    #[must_use]
    pub fn trigger_label(&self) -> &str {
        &self.trigger_label
    }

    #[must_use]
    pub fn candidate_title(&self) -> Option<&str> {
        self.candidate_title.as_deref()
    }

    #[must_use]
    pub fn candidate_subtitle(&self) -> Option<&str> {
        self.candidate_subtitle.as_deref()
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    #[must_use]
    pub fn options(&self) -> &[MusicBrainzCandidateOptionVm] {
        &self.options
    }

    #[must_use]
    pub fn has_candidates(&self) -> bool {
        !self.options.is_empty()
    }
}

fn selected_candidate_index(
    result: &MusicBrainzLookupResult,
    requested_index: usize,
) -> Option<usize> {
    if result.lookup.candidates.is_empty() {
        None
    } else if requested_index < result.lookup.candidates.len() {
        Some(requested_index)
    } else {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::musicbrainz::{MusicBrainzCandidate, MusicBrainzLookup};

    fn result(candidates: Vec<MusicBrainzCandidate>) -> MusicBrainzLookupResult {
        MusicBrainzLookupResult {
            lookup: MusicBrainzLookup {
                query: "q".into(),
                candidates,
            },
            image: None,
        }
    }

    fn candidate(title: &str, release_id: &str, country: &str) -> MusicBrainzCandidate {
        MusicBrainzCandidate {
            title: title.into(),
            release_id: Some(release_id.into()),
            country: Some(country.into()),
            total_tracks: Some(10),
            similarity_score: 90,
            ..MusicBrainzCandidate::default()
        }
    }

    #[test]
    fn empty_lookup_projects_disabled_no_release_state() {
        let vm = MusicBrainzPanelVm::new(&result(Vec::new()), 0);
        assert_eq!(vm.trigger_label(), "MusicBrainz: No MusicBrainz release");
        assert_eq!(vm.candidate_title(), None);
        assert_eq!(vm.candidate_subtitle(), None);
        assert_eq!(vm.selected_index(), None);
        assert!(!vm.has_candidates());
    }

    #[test]
    fn selected_candidate_projects_title_subtitle_and_options() {
        let lookup = result(vec![
            candidate("First", "r1", "US"),
            candidate("Second", "r2", "CA"),
        ]);
        let vm = MusicBrainzPanelVm::new(&lookup, 1);

        assert!(vm.trigger_label().contains("CA"));
        assert_eq!(vm.candidate_title(), Some("Second"));
        assert!(vm
            .candidate_subtitle()
            .is_some_and(|value| value.contains("#2")));
        assert_eq!(vm.selected_index(), Some(1));
        assert_eq!(vm.options().len(), 2);
        assert!(!vm.options()[0].selected);
        assert!(vm.options()[1].selected);
    }

    #[test]
    fn invalid_selection_falls_back_to_first_candidate() {
        let lookup = result(vec![candidate("First", "r1", "US")]);
        let vm = MusicBrainzPanelVm::new(&lookup, 99);

        assert_eq!(vm.candidate_title(), Some("First"));
        assert_eq!(vm.selected_index(), Some(0));
        assert!(vm
            .candidate_subtitle()
            .is_some_and(|value| value.contains("#1")));
    }
}
