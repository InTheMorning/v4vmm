//! Shared pagination display policy.
//!
//! GPUI shells consume these pure helpers to decide when to request more
//! rows and how much pending content to show while a page is loading.

#![warn(clippy::pedantic)]

/// Distance from the bottom of a scrollable list, in pixels, at which
/// auto-pagination should fire.
pub(crate) const AUTO_PAGINATE_THRESHOLD_PX: f32 = 240.0;

/// Number of skeleton placeholders to paint when a list is loading
/// from cold.
pub(crate) const INITIAL_SKELETON_COUNT: usize = 6;

/// Number of skeleton placeholders to paint at the tail of a list that
/// is currently fetching the next page.
pub(crate) const TAIL_SKELETON_COUNT: usize = 3;

/// Pure skeleton-count policy for loading paged rows or tiles.
#[must_use]
pub(crate) fn pending_skeleton_count(loading: bool, has_existing_rows: bool) -> usize {
    if !loading {
        0
    } else if has_existing_rows {
        TAIL_SKELETON_COUNT
    } else {
        INITIAL_SKELETON_COUNT
    }
}

/// Pure pagination policy for triggering a "load more" request.
///
/// `remaining_px` is the unscrolled distance to the bottom of the list.
#[must_use]
pub(crate) fn should_auto_load_more(
    remaining_px: f32,
    threshold_px: f32,
    has_more: bool,
    loading: bool,
) -> bool {
    has_more && !loading && remaining_px.is_finite() && remaining_px <= threshold_px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_load_more_fires_only_when_near_bottom_with_more_pages_and_idle() {
        assert!(!should_auto_load_more(800.0, 240.0, true, false));
        assert!(should_auto_load_more(120.0, 240.0, true, false));
        assert!(should_auto_load_more(240.0, 240.0, true, false));
        assert!(!should_auto_load_more(50.0, 240.0, false, false));
        assert!(!should_auto_load_more(50.0, 240.0, true, true));
        assert!(!should_auto_load_more(f32::NAN, 240.0, true, false));
        assert!(!should_auto_load_more(f32::INFINITY, 240.0, true, false));
    }

    #[test]
    fn pending_skeleton_count_paints_initial_grid_on_cold_load() {
        assert_eq!(
            pending_skeleton_count(true, false),
            INITIAL_SKELETON_COUNT,
            "cold load with no rows yet should paint the full skeleton grid"
        );
    }

    #[test]
    fn pending_skeleton_count_paints_tail_when_appending() {
        assert_eq!(
            pending_skeleton_count(true, true),
            TAIL_SKELETON_COUNT,
            "appending more rows should paint a short tail of skeletons"
        );
    }

    #[test]
    fn pending_skeleton_count_paints_nothing_when_idle() {
        assert_eq!(pending_skeleton_count(false, false), 0);
        assert_eq!(pending_skeleton_count(false, true), 0);
    }

    #[test]
    fn skeleton_counts_are_sane() {
        const _: () = assert!(INITIAL_SKELETON_COUNT >= TAIL_SKELETON_COUNT);
        const _: () = assert!(TAIL_SKELETON_COUNT >= 1);
        const _: () = assert!(INITIAL_SKELETON_COUNT <= 12);
    }
}
