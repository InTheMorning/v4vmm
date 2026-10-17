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

/// Pure near-end pagination decision from raw scroll geometry (ADR 0084
/// Decision 1, amended 2026-10-17, task 014). A wheel scroll, a touchpad
/// gesture and a scrollbar drag all produce this same geometry, so one
/// function starts the next page load for each of the three.
///
/// `scroll_offset_y` is the scroll handle's own offset: zero at the top,
/// more negative the further the column scrolls down. `viewport_height`
/// and `content_height` are the column's own visible height and its full
/// content height.
#[must_use]
pub(crate) fn should_load_more_near_scroll_end(
    scroll_offset_y: f32,
    viewport_height: f32,
    content_height: f32,
    has_more: bool,
    is_loading: bool,
) -> bool {
    // Before its first layout the column has no height. That is not the
    // end of the content, so it loads no page.
    if viewport_height <= 0.0 {
        return false;
    }
    let max_offset_y = (content_height - viewport_height).max(0.0);
    let remaining_px = max_offset_y + scroll_offset_y;
    should_auto_load_more(
        remaining_px,
        AUTO_PAGINATE_THRESHOLD_PX,
        has_more,
        is_loading,
    )
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

    /// R14-02: scrolled to the end, with more pages and no page in
    /// flight, the column loads the next page. A viewport of 600px and
    /// content of 1000px leaves 400px to scroll.
    #[test]
    fn adr_0084_scrollbars_near_scroll_end_loads_with_more_pages_and_idle() {
        assert!(should_load_more_near_scroll_end(
            -400.0, 600.0, 1000.0, true, false
        ));
    }

    /// R14-02: at the top of long content, the column does not load.
    #[test]
    fn adr_0084_scrollbars_top_of_long_content_does_not_load() {
        assert!(!should_load_more_near_scroll_end(
            0.0, 600.0, 1000.0, true, false
        ));
    }

    /// R14-02: a page already in flight blocks a second request, even at
    /// the end of the column.
    #[test]
    fn adr_0084_scrollbars_near_scroll_end_waits_for_a_page_in_flight() {
        assert!(!should_load_more_near_scroll_end(
            -400.0, 600.0, 1000.0, true, true
        ));
    }

    /// R14-02: with no more pages, the end of the column never loads.
    #[test]
    fn adr_0084_scrollbars_near_scroll_end_stops_with_no_more_pages() {
        assert!(!should_load_more_near_scroll_end(
            -400.0, 600.0, 1000.0, false, false
        ));
    }

    /// R14-02: before its first layout the column has no height, so it
    /// loads no page.
    #[test]
    fn adr_0084_scrollbars_no_layout_yet_does_not_load() {
        assert!(!should_load_more_near_scroll_end(
            0.0, 0.0, 0.0, true, false
        ));
    }

    /// R14-02: content shorter than the viewport leaves no distance to
    /// scroll, so the column is already at its end.
    #[test]
    fn adr_0084_scrollbars_short_content_has_no_distance_left_to_scroll() {
        assert!(should_load_more_near_scroll_end(
            0.0, 600.0, 400.0, true, false
        ));
    }

    /// R14-02: the decision takes only the five named inputs, so a wheel
    /// scroll, a touchpad gesture and a scrollbar drag reach the same
    /// answer from the same geometry.
    #[test]
    fn adr_0084_scrollbars_near_scroll_end_is_a_pure_function_of_its_inputs() {
        let first = should_load_more_near_scroll_end(-400.0, 600.0, 1000.0, true, false);
        let second = should_load_more_near_scroll_end(-400.0, 600.0, 1000.0, true, false);
        assert_eq!(first, second);
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
