use gpui::{Hsla, black, white};

use super::{GradientStop, added, color_at, moved};

fn stops() -> Vec<GradientStop> {
    vec![
        GradientStop {
            at: 0.0,
            color: black(),
        },
        GradientStop {
            at: 1.0,
            color: white(),
        },
    ]
}

#[test]
fn colors_blend_between_stops() {
    let middle: Hsla = color_at(&stops(), 0.5);
    assert!((middle.l - 0.5).abs() < 0.05);
    assert_eq!(color_at(&stops(), -1.0), black());
}

#[test]
fn adding_and_moving_keep_stops_sorted() {
    let (three, ix) = added(&stops(), 0.25);
    assert_eq!((three.len(), ix, three[1].at), (3, 1, 0.25));
    let (swapped, landed) = moved(&three, 0, 0.5);
    assert_eq!(landed, 1);
    assert_eq!(
        swapped.iter().map(|stop| stop.at).collect::<Vec<_>>(),
        [0.25, 0.5, 1.0]
    );
}

#[test]
fn zero_distance_movement_keeps_equal_stops_in_order() {
    let equal = stops().into_iter().map(|mut stop| {
        stop.at = 0.5;
        stop
    });
    let original = vec![equal.clone().next().unwrap(), equal.clone().nth(1).unwrap()];
    let (same, landed) = moved(&original, 0, 0.5);
    assert_eq!(landed, 0);
    assert_eq!(same, original);
}
