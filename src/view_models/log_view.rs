//! Per-source log reading, exact line anchors and typed following actions (ADR 0063).

#![warn(clippy::pedantic)]

/// Identity is independent of a report's title, current result, or revision.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum LogSource {
    Startup,
    Session,
    Configuration,
    Background,
    Service { host: String, unit: String },
    Event { endpoint: String, event: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FollowAvailability {
    Available,
    AlreadyFollowing,
}

pub(crate) struct FollowAction {
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: FollowAvailability,
}

#[derive(Debug, Default)]
pub(crate) struct LogReadingVm {
    text: String,
    paused: bool,
    top_line: usize,
    line_fraction: f32,
    anchor_lost: bool,
}

impl LogReadingVm {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn following(&self) -> bool {
        !self.paused
    }

    pub(crate) fn action(&self) -> FollowAction {
        FollowAction {
            label: "Go to latest",
            a11y_label: "Go to the latest log entry and resume following",
            availability: if self.paused {
                FollowAvailability::Available
            } else {
                FollowAvailability::AlreadyFollowing
            },
        }
    }

    pub(crate) fn status(&self) -> &'static str {
        if self.anchor_lost {
            "Earlier text is no longer available. Showing the oldest available entry."
        } else if self.paused {
            "Following paused"
        } else {
            "Following latest entries"
        }
    }

    /// Called only for a user scroll, not for a layout or content-driven offset.
    pub(crate) fn scroll(&mut self, top: f32, maximum: f32, line_height: f32) {
        if !top.is_finite()
            || !maximum.is_finite()
            || !line_height.is_finite()
            || line_height <= 0.0
        {
            return;
        }
        let top = top.clamp(0.0, maximum.max(0.0));
        self.paused = top + 1.0 < maximum;
        let line_position = top / line_height;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "finite nonnegative viewport position is bounded by the rendered text"
        )]
        {
            self.top_line = line_position.floor() as usize;
        }
        self.line_fraction = line_position.fract();
        self.anchor_lost = false;
    }

    pub(crate) fn latest(&mut self) {
        self.paused = false;
        self.anchor_lost = false;
    }

    /// Keep an unchanged logical line, including duplicate-line occurrence, through snapshots.
    pub(crate) fn replace(&mut self, text: &str) -> bool {
        if self.text == text {
            return false;
        }
        if self.paused && !text.starts_with(&self.text) {
            let old_lines: Vec<_> = self.text.lines().collect();
            let new_lines: Vec<_> = text.lines().collect();
            let anchor = old_lines.get(self.top_line);
            // Prefer the matching suffix: finite journal snapshots commonly trim their head.
            let overlap = (1..=old_lines.len().min(new_lines.len()))
                .rev()
                .find(|&count| old_lines[old_lines.len() - count..] == new_lines[..count]);
            let relocated = overlap
                .and_then(|count| self.top_line.checked_sub(old_lines.len() - count))
                .or_else(|| {
                    let anchor = anchor?;
                    let occurrence = old_lines[..self.top_line]
                        .iter()
                        .filter(|line| *line == anchor)
                        .count();
                    new_lines
                        .iter()
                        .enumerate()
                        .filter(|(_, line)| *line == anchor)
                        .nth(occurrence)
                        .map(|(index, _)| index)
                });
            if let Some(line) = relocated {
                self.top_line = line;
            } else {
                self.top_line = 0;
                self.line_fraction = 0.0;
                self.anchor_lost = true;
            }
        }
        text.clone_into(&mut self.text);
        true
    }

    pub(crate) fn reading_top(&self, line_height: f32) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "viewport geometry uses f32; line count is bounded by the displayed report"
        )]
        {
            (self.top_line as f32 + self.line_fraction) * line_height
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0063_follow_pause_append_and_return_are_explicit() {
        let mut vm = LogReadingVm::default();
        assert!(vm.following());
        vm.replace("first\nsecond\nthird");
        vm.scroll(15.0, 100.0, 10.0);
        assert!(!vm.following());
        assert_eq!(vm.action().availability, FollowAvailability::Available);
        vm.replace("first\nsecond\nthird\nfourth");
        assert_eq!(vm.reading_top(10.0), 15.0);
        assert!(!vm.following());
        vm.scroll(100.0, 100.0, 10.0);
        assert!(vm.following());
        vm.scroll(0.0, 100.0, 10.0);
        vm.latest();
        assert!(vm.following());
        assert_eq!(
            vm.action().availability,
            FollowAvailability::AlreadyFollowing
        );
    }

    #[test]
    fn adr_0063_trim_preserves_surviving_unicode_anchor_and_explains_loss() {
        let mut vm = LogReadingVm::default();
        vm.replace("old\nrépété\nrépété\n🦀\ntail");
        vm.scroll(25.0, 100.0, 10.0);
        vm.replace("répété\nrépété\n🦀\ntail\nnew");
        assert_eq!(vm.reading_top(10.0), 15.0);
        assert_eq!(vm.reading_top(20.0), 30.0);
        vm.replace("different\nreplacement");
        assert!(!vm.following());
        assert_eq!(vm.reading_top(10.0), 0.0);
        assert!(vm.status().contains("no longer available"));
        assert!(!vm.replace("different\nreplacement"));
        vm.latest();
        assert!(!vm.status().contains("no longer available"));
    }

    #[test]
    fn adr_0063_invalid_geometry_does_not_change_following() {
        let mut vm = LogReadingVm::default();
        vm.scroll(f32::NAN, 10.0, 1.0);
        assert!(vm.following());
    }
}
