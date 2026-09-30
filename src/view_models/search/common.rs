//! Shared helpers for Search view-model projections.

#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ADR 0060 packet 006 deletes this parked query layer"
    )
)]

pub(super) fn nonempty_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| {
        !value.is_empty()
            && value
                .chars()
                .any(|ch| ch != '.' && ch != '\u{2026}' && !ch.is_whitespace())
    })
}
