use super::*;
use gpui::{px, size};

#[test]
fn collapsed_axes_defer_independently_without_requesting_frames() {
    let metrics = Metrics {
        wide: px(96.),
        tall: px(48.),
        label: px(60.),
        frozen: point(px(96.), px(48.)),
    };
    let bounds = |w, h| Bounds::new(point(px(0.), px(0.)), size(px(w), px(h)));
    let mut sheet = Sheet {
        cursor: (4, 3),
        anchor: (4, 3),
        reveal_pending: (true, true),
        ..Default::default()
    };
    let collapsed = bounds(120., 80.);
    assert!(
        measured(&mut sheet, collapsed, metrics, (1, 1)),
        "new bounds need one layout frame"
    );
    assert_eq!(sheet.reveal_pending, (true, true));
    assert_eq!(sheet.offset, point(px(0.), px(0.)));
    for _ in 0..3 {
        assert!(
            !measured(&mut sheet, collapsed, metrics, (1, 1)),
            "unchanged collapse must not schedule a frame"
        );
    }
    // Positive but sub-cell room is still insufficient for full containment.
    assert!(measured(&mut sheet, bounds(200., 120.), metrics, (1, 1)));
    assert_eq!(sheet.reveal_pending, (true, true));
    assert_eq!(sheet.offset, point(px(0.), px(0.)));
    let width_restored = bounds(300., 80.);
    assert!(measured(&mut sheet, width_restored, metrics, (1, 1)));
    assert_eq!(sheet.reveal_pending, (false, true));
    assert_eq!(sheet.offset, point(px(144.), px(0.)));
    assert!(!measured(&mut sheet, width_restored, metrics, (1, 1)));
    // The resolved axis belongs to the user again even while Y remains pending.
    sheet.offset.x = px(100.);
    assert!(!measured(&mut sheet, width_restored, metrics, (1, 1)));
    assert!(measured(&mut sheet, bounds(300., 160.), metrics, (1, 1)));
    assert_eq!(sheet.reveal_pending, (false, false));
    assert_eq!(sheet.offset, point(px(100.), px(128.)));
    assert!(!measured(&mut sheet, bounds(300., 160.), metrics, (1, 1)));
}

#[test]
fn pending_reveal_uses_changed_metrics_even_with_unchanged_bounds() {
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(180.), px(120.)));
    let mut metrics = Metrics {
        wide: px(96.),
        tall: px(48.),
        label: px(60.),
        frozen: point(px(96.), px(48.)),
    };
    let mut sheet = Sheet {
        cursor: (2, 2),
        reveal_pending: (true, true),
        ..Default::default()
    };
    measured(&mut sheet, bounds, metrics, (1, 1));
    assert_eq!(sheet.reveal_pending, (true, true));
    metrics.frozen = point(px(0.), px(0.));
    assert!(measured(&mut sheet, bounds, metrics, (0, 0)));
    assert_eq!(sheet.reveal_pending, (false, false));
    assert_eq!(sheet.offset, point(px(168.), px(72.)));
}
