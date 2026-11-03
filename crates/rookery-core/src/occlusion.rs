//! Occlusion coverage math.
//!
//! OS probes enumerate on-screen windows and hand their bounds to [`is_covered`] /
//! [`coverage`], which computes the exact area of the union of rectangles clipped to a
//! display using a coordinate-compressed sweep. This keeps the tricky geometry in one
//! well-tested place.

use crate::geometry::Rect;

/// Default fraction of a display that must be covered before it counts as occluded.
/// Slightly below 1.0 so that thin gaps (menu bar insets, rounded corners, 1px borders)
/// don't keep the simulation running behind a maximized window.
pub const DEFAULT_COVERAGE_THRESHOLD: f64 = 0.97;

/// Fraction (0.0–1.0) of `display` covered by the union of `windows`.
pub fn coverage(display: Rect, windows: &[Rect]) -> f64 {
    let display_area = display.area();
    if display_area <= 0.0 {
        return 0.0;
    }

    let clipped: Vec<Rect> = windows
        .iter()
        .filter_map(|w| w.intersect(&display))
        .collect();
    if clipped.is_empty() {
        return 0.0;
    }

    // Coordinate-compressed sweep over x; for each slab, merge y-intervals.
    let mut xs: Vec<f64> = clipped.iter().flat_map(|r| [r.x, r.right()]).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();

    let mut covered = 0.0;
    let mut intervals: Vec<(f64, f64)> = Vec::with_capacity(clipped.len());
    for slab in xs.windows(2) {
        let (x0, x1) = (slab[0], slab[1]);
        let width = x1 - x0;
        if width <= 0.0 {
            continue;
        }
        intervals.clear();
        intervals.extend(
            clipped
                .iter()
                .filter(|r| r.x <= x0 && r.right() >= x1)
                .map(|r| (r.y, r.bottom())),
        );
        if intervals.is_empty() {
            continue;
        }
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged_len = 0.0;
        let (mut cur_start, mut cur_end) = intervals[0];
        for &(start, end) in &intervals[1..] {
            if start > cur_end {
                merged_len += cur_end - cur_start;
                cur_start = start;
                cur_end = end;
            } else if end > cur_end {
                cur_end = end;
            }
        }
        merged_len += cur_end - cur_start;
        covered += merged_len * width;
    }

    (covered / display_area).clamp(0.0, 1.0)
}

/// True if the union of `windows` covers at least `threshold` of `display`.
pub fn is_covered(display: Rect, windows: &[Rect], threshold: f64) -> bool {
    coverage(display, windows) >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPLAY: Rect = Rect::new(0.0, 0.0, 1920.0, 1080.0);

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn no_windows_is_uncovered() {
        assert_eq!(coverage(DISPLAY, &[]), 0.0);
        assert!(!is_covered(DISPLAY, &[], DEFAULT_COVERAGE_THRESHOLD));
    }

    #[test]
    fn maximized_window_covers_display() {
        let w = [Rect::new(0.0, 0.0, 1920.0, 1080.0)];
        assert!(approx(coverage(DISPLAY, &w), 1.0));
        assert!(is_covered(DISPLAY, &w, DEFAULT_COVERAGE_THRESHOLD));
    }

    #[test]
    fn maximized_below_menu_bar_still_counts() {
        // macOS: 25pt menu bar not covered -> 1055/1080 = 97.7%
        let w = [Rect::new(0.0, 25.0, 1920.0, 1055.0)];
        assert!(is_covered(DISPLAY, &w, DEFAULT_COVERAGE_THRESHOLD));
    }

    #[test]
    fn half_screen_window_is_half_covered() {
        let w = [Rect::new(0.0, 0.0, 960.0, 1080.0)];
        assert!(approx(coverage(DISPLAY, &w), 0.5));
        assert!(!is_covered(DISPLAY, &w, DEFAULT_COVERAGE_THRESHOLD));
    }

    #[test]
    fn two_snapped_halves_cover_display() {
        let w = [
            Rect::new(0.0, 0.0, 960.0, 1080.0),
            Rect::new(960.0, 0.0, 960.0, 1080.0),
        ];
        assert!(approx(coverage(DISPLAY, &w), 1.0));
    }

    #[test]
    fn overlapping_windows_are_not_double_counted() {
        let w = [
            Rect::new(0.0, 0.0, 1200.0, 1080.0),
            Rect::new(700.0, 0.0, 1220.0, 1080.0),
        ];
        assert!(approx(coverage(DISPLAY, &w), 1.0));
        let w2 = [
            Rect::new(0.0, 0.0, 960.0, 540.0),
            Rect::new(0.0, 0.0, 960.0, 540.0),
        ];
        assert!(approx(coverage(DISPLAY, &w2), 0.25));
    }

    #[test]
    fn windows_on_other_display_are_ignored() {
        let w = [Rect::new(1920.0, 0.0, 2560.0, 1440.0)];
        assert_eq!(coverage(DISPLAY, &w), 0.0);
    }

    #[test]
    fn window_spanning_displays_is_clipped() {
        let w = [Rect::new(-100.0, -100.0, 5000.0, 5000.0)];
        assert!(approx(coverage(DISPLAY, &w), 1.0));
    }

    #[test]
    fn hole_in_the_middle_is_detected() {
        // Four windows framing a 200x200 hole in the middle.
        let w = [
            Rect::new(0.0, 0.0, 1920.0, 440.0),
            Rect::new(0.0, 640.0, 1920.0, 440.0),
            Rect::new(0.0, 440.0, 860.0, 200.0),
            Rect::new(1060.0, 440.0, 860.0, 200.0),
        ];
        let expected = 1.0 - (200.0 * 200.0) / (1920.0 * 1080.0);
        assert!(approx(coverage(DISPLAY, &w), expected));
    }

    #[test]
    fn offset_display_origin() {
        let display = Rect::new(-2560.0, 0.0, 2560.0, 1440.0);
        let w = [Rect::new(-2560.0, 0.0, 2560.0, 1440.0)];
        assert!(approx(coverage(display, &w), 1.0));
    }

    #[test]
    fn degenerate_display() {
        let display = Rect::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(coverage(display, &[DISPLAY]), 0.0);
    }
}
