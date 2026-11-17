use super::*;

struct TestMeasurer;

impl TextMeasurer for TestMeasurer {
    fn measure(&self, text: &str, size: f32) -> (f32, f32) {
        (text.len() as f32 * size, size)
    }
}

#[test]
fn linear_ticks_generate_major() {
    let axis = AxisConfig::new();
    let ticks = generate_ticks(&axis, Range::new(0.0, 10.0), 400.0);
    assert!(ticks.iter().any(|tick| tick.is_major));
}

#[test]
fn linear_ticks_stop_when_step_cannot_advance_large_values() {
    let axis = AxisConfig::new();
    let ticks = generate_linear_ticks(&axis, Range::new(1e20, 1e20 + 16384.0), 1000.0);
    assert_eq!(ticks.iter().filter(|tick| tick.is_major).count(), 1);
}

#[test]
fn layout_cache_refreshes_labels_when_formatter_changes() {
    let mut cache = AxisLayoutCache::default();
    let range = Range::new(0.0, 10.0);
    let first = AxisConfig::new();
    assert!(
        cache
            .update(&first, range, 400, &TestMeasurer)
            .ticks
            .iter()
            .any(|tick| tick.label == "0")
    );

    let second = AxisConfig::builder()
        .formatter(AxisFormatter::Custom(Arc::new(|value| {
            format!("new:{value:.0}")
        })))
        .build();
    assert!(
        cache
            .update(&second, range, 400, &TestMeasurer)
            .ticks
            .iter()
            .any(|tick| tick.label == "new:0")
    );
}

#[test]
fn layout_cache_remeasures_when_label_size_changes() {
    let mut cache = AxisLayoutCache::default();
    let range = Range::new(0.0, 10.0);
    let small = AxisConfig::builder().label_size(10.0).build();
    let small_size = cache
        .update(&small, range, 400, &TestMeasurer)
        .max_label_size;
    let large = AxisConfig::builder().label_size(20.0).build();
    let large_size = cache
        .update(&large, range, 400, &TestMeasurer)
        .max_label_size;
    assert_eq!(large_size.0, small_size.0 * 2.0);
    assert_eq!(large_size.1, small_size.1 * 2.0);
}

#[test]
fn log_and_symlog_ticks_cover_range() {
    let log_axis = AxisConfig::builder().log(10.0).build();
    let ticks = generate_ticks(&log_axis, Range::new(1.0, 1000.0), 400.0);
    let majors: Vec<f64> = ticks
        .iter()
        .filter(|t| t.is_major)
        .map(|t| t.value)
        .collect();
    assert_eq!(majors, vec![1.0, 10.0, 100.0, 1000.0]);

    let sym_axis = AxisConfig::builder().symlog(1.0).build();
    let sym_ticks = generate_ticks(&sym_axis, Range::new(-100.0, 100.0), 400.0);
    assert!(sym_ticks.iter().any(|t| t.value == 0.0));
    assert!(sym_ticks.iter().any(|t| t.value < 0.0));
    assert!(sym_ticks.iter().any(|t| t.value > 0.0));
}

#[test]
fn time_ticks_align_to_kst_round_hours_and_five_minutes() {
    let kst = 9 * 3600;
    let axis = AxisConfig::builder().time_offset_seconds(kst).build();
    let start = 1_794_517_200.0;
    let ticks = generate_ticks(&axis, Range::new(start, start + 43200.0), 800.0);
    let majors: Vec<f64> = ticks
        .iter()
        .filter(|t| t.is_major)
        .map(|t| t.value)
        .collect();
    assert!(!majors.is_empty());
    assert!(
        majors
            .iter()
            .all(|v| ((*v as i64) + i64::from(kst)).rem_euclid(3600) == 0)
    );

    let zoomed = generate_ticks(&axis, Range::new(start, start + 1800.0), 800.0);
    let zoom_majors: Vec<f64> = zoomed
        .iter()
        .filter(|t| t.is_major)
        .map(|t| t.value)
        .collect();
    assert!(!zoom_majors.is_empty());
    assert!(
        zoom_majors
            .iter()
            .all(|v| ((*v as i64) + i64::from(kst)).rem_euclid(300) == 0)
    );
}
