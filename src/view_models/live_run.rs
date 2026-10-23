//! The pause and the run generation of a live search (ADR 0087 Decision 4,
//! packet 003).
//!
//! A search runs after the curator stops typing for a pause. Each change of
//! the text gives a new [`PauseTicket`]. Only the ticket of the last change
//! can end the pause. Each run gets a new [`RunGeneration`], and only the
//! newest generation is current. This keeps the ADR 0087 invariant "A result
//! of an older run never replaces the result of a newer run".
//!
//! [`LiveRunGate`] is a pure state machine. It holds no GPUI type and reads
//! no clock. The host starts one timer for each ticket, and gives the ticket
//! back when the timer ends.

#![warn(clippy::pedantic)]

/// The ticket of one change of the text. Only the ticket of the last change
/// can end the pause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PauseTicket(u64);

/// The generation of one run. Only the newest generation is current.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RunGeneration(u64);

/// The pause counter and the run counter of one search surface.
#[derive(Debug, Default)]
pub(crate) struct LiveRunGate {
    /// The ticket number of the last change of the text.
    last_ticket: u64,
    /// `true` while the pause of the last change can still end.
    pause_armed: bool,
    /// The number of the last run.
    last_run: u64,
}

impl LiveRunGate {
    /// Records one change of the text. The pause is armed, and the function
    /// gives the ticket of this change.
    pub(crate) fn text_changed(&mut self) -> PauseTicket {
        self.last_ticket += 1;
        self.pause_armed = true;
        PauseTicket(self.last_ticket)
    }

    /// Gives `true` one time, when the pause is armed and `ticket` is the
    /// ticket of the last change. The pause is then not armed. Each other
    /// ticket gives `false`.
    pub(crate) fn pause_ended(&mut self, ticket: PauseTicket) -> bool {
        if !self.pause_armed || ticket.0 != self.last_ticket {
            return false;
        }
        self.pause_armed = false;
        true
    }

    /// Stops the pause. A later `pause_ended` gives `false` until the next
    /// change of the text.
    pub(crate) fn cancel_pause(&mut self) {
        self.pause_armed = false;
    }

    /// Starts a new run and gives its generation. The pause is not armed,
    /// so the pause of an earlier change does not run the text again.
    pub(crate) fn next_run(&mut self) -> RunGeneration {
        self.last_run += 1;
        self.pause_armed = false;
        RunGeneration(self.last_run)
    }

    /// `true` only for the last generation that `next_run` gave.
    #[must_use]
    pub(crate) fn is_current(&self, generation: RunGeneration) -> bool {
        generation.0 == self.last_run
    }

    /// The last generation that `next_run` gave.
    /// No production code calls it yet (ADR 0087 packet 003).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn current_run(&self) -> RunGeneration {
        RunGeneration(self.last_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R87-3-01: only the ticket of the last change ends the pause, and it
    /// ends the pause one time.
    #[test]
    fn adr_0087_live_results_r87_3_01_last_ticket_ends_the_pause_once() {
        let mut gate = LiveRunGate::default();
        let a = gate.text_changed();
        let b = gate.text_changed();
        assert!(!gate.pause_ended(a));
        assert!(gate.pause_ended(b));
        assert!(!gate.pause_ended(b));
    }

    /// R87-3-02: a run at once stops the pause, so the pause does not run
    /// the text a second time.
    #[test]
    fn adr_0087_live_results_r87_3_02_run_at_once_stops_the_pause() {
        let mut gate = LiveRunGate::default();
        let a = gate.text_changed();
        let generation = gate.next_run();
        assert!(gate.is_current(generation));
        assert!(!gate.pause_ended(a));
    }

    /// R87-3-03: `cancel_pause` stops the pause. A later change arms it
    /// again.
    #[test]
    fn adr_0087_live_results_r87_3_03_cancel_then_arm_again() {
        let mut gate = LiveRunGate::default();
        let a = gate.text_changed();
        gate.cancel_pause();
        assert!(!gate.pause_ended(a));
        let b = gate.text_changed();
        assert!(gate.pause_ended(b));
    }

    /// R87-3-04: the generations increase, and only the last one is
    /// current.
    #[test]
    fn adr_0087_live_results_r87_3_04_only_the_newest_generation_is_current() {
        let mut gate = LiveRunGate::default();
        let first = gate.next_run();
        let second = gate.next_run();
        let third = gate.next_run();
        assert!(first < second && second < third);
        assert!(!gate.is_current(first));
        assert!(!gate.is_current(second));
        assert!(gate.is_current(third));
        assert_eq!(gate.current_run(), third);
    }
}
