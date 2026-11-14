use super::*;

#[test]
fn linear_ticks_generate_major() {
    let axis = AxisConfig::new();
    let ticks = generate_ticks(&axis, Range::new(0.0, 10.0), 400.0);
    assert!(ticks.iter().any(|tick| tick.is_major));
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
