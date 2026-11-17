use super::*;

fn chart<'a>(
    labels: &'a [SharedString],
    values: &'a [Vec<f64>],
    hidden: &'a HashSet<SharedString>,
    mark: Mark,
    stacked: bool,
) -> Categories<'a> {
    static NAMES: [SharedString; 2] = [
        SharedString::new_static("North"),
        SharedString::new_static("South"),
    ];
    Categories {
        labels,
        names: &NAMES,
        values,
        mark,
        stacked,
        stream: false,
        horizontal: false,
        gap: 0.28,
        y_scale: ScaleMode::Linear,
        hidden,
        span: (0, 2),
    }
}

fn sales() -> Vec<Vec<f64>> {
    vec![vec![10.0, 20.0, 30.0], vec![5.0, 5.0, 10.0]]
}

#[test]
fn stacked_bars_share_a_band_and_grouped_bars_split_it() {
    let labels = ["Jan", "Feb", "Mar"].map(SharedString::from);
    let (values, hidden) = (sales(), HashSet::new());
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 100.0,
    };
    let stack = categories(&chart(&labels, &values, &hidden, Mark::Bar, true), frame);
    assert_eq!(stack.bars.len(), 6);
    assert_eq!(
        stack.bars[0].1.x, stack.bars[3].1.x,
        "a stack shares its band"
    );
    assert_eq!(
        stack.bars[3].1.y + stack.bars[3].1.h,
        stack.bars[0].1.y,
        "the second sits on the first"
    );
    let group = categories(&chart(&labels, &values, &hidden, Mark::Bar, false), frame);
    assert!(
        group.bars[3].1.x > group.bars[0].1.x + group.bars[0].1.w,
        "grouped bars stand apart"
    );
    assert_eq!(group.ticks.first().map(|tick| tick.1), Some(0.0));
}

#[test]
fn lines_run_edge_to_edge_and_hidden_series_drop_out() {
    let labels = ["Jan", "Feb", "Mar"].map(SharedString::from);
    let values = sales();
    let hidden: HashSet<SharedString> = ["South".into()].into_iter().collect();
    let frame = Rect {
        x: 10.0,
        y: 0.0,
        w: 200.0,
        h: 100.0,
    };
    let lines = categories(&chart(&labels, &values, &hidden, Mark::Line, false), frame);
    assert_eq!(lines.lines.len(), 1);
    let xs: Vec<f32> = lines.lines[0].points.iter().map(|point| point.0).collect();
    assert_eq!(xs, [10.0, 110.0, 210.0]);
    assert_eq!(
        lines.lines[0].points[2].1, 0.0,
        "the highest value meets the top of the rounded scale"
    );
    assert_eq!(lines.lines[0].floor, None, "a line has no floor");
}

#[test]
fn bubbles_grow_by_area_and_keep_their_keys_past_a_hidden_cloud() {
    let first = Points::new("Towns", [(1.0, 1.0)]);
    let cloud = Points::new("Cities", [(0.0, 0.0), (10.0, 10.0)]).sizes([1.0, 4.0]);
    let hidden: HashSet<SharedString> = ["Towns".into()].into_iter().collect();
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
    };
    let geometry = scatter(
        &[first, cloud],
        &hidden,
        frame,
        (20.0, 4.0),
        (ScaleMode::Linear, ScaleMode::Linear),
        None,
        false,
        &|value| value.to_string(),
    );
    assert_eq!(geometry.dots[1].radius, 20.0);
    assert_eq!(
        geometry.dots[0].radius, 10.0,
        "a quarter of the area, half the radius"
    );
    assert_eq!((geometry.dots[0].key, geometry.dots[1].key), (1, 2));
    assert_eq!(
        geometry.labels.first().map(|label| label.1.clone()),
        Some("0".into())
    );
}

#[test]
fn no_categories_lay_out_nothing() {
    let hidden = HashSet::new();
    let empty = Categories {
        labels: &[],
        names: &[],
        values: &[],
        mark: Mark::Line,
        stacked: false,
        stream: false,
        horizontal: false,
        gap: 0.28,
        y_scale: ScaleMode::Linear,
        hidden: &hidden,
        span: (0, 0),
    };
    assert_eq!(
        categories(
            &empty,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0
            }
        ),
        Geometry::default()
    );
}

#[test]
fn a_frame_leaves_its_gutters() {
    let rect = frame((400.0, 200.0), 48.0, 24.0, 8.0);
    assert_eq!(
        rect,
        Rect {
            x: 48.0,
            y: 8.0,
            w: 344.0,
            h: 168.0
        }
    );
    assert!(rect.contains((48.0, 8.0)) && !rect.contains((40.0, 50.0)));
    assert_eq!(
        frame((10.0, 10.0), 48.0, 24.0, 8.0).w,
        0.0,
        "a small box leaves no room"
    );
}
