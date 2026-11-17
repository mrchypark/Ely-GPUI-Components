use super::{fraction, keyed, value_at};

#[test]
fn values_snap_to_the_step_and_stay_in_range() {
    assert_eq!(fraction(25.0, 0.0, 100.0), 0.25);
    assert_eq!(value_at(0.26, 0.0, 100.0, 5.0), 25.0);
    assert_eq!(value_at(1.4, 0.0, 100.0, 5.0), 100.0);
    assert_eq!(value_at(0.5, -1.0, 1.0, 0.5), 0.0);
}

#[test]
fn keys_step_jump_and_reach_the_ends() {
    assert_eq!(keyed("right", 10.0, 0.0, 100.0, 1.0), Some(11.0));
    assert_eq!(keyed("pagedown", 5.0, 0.0, 100.0, 1.0), Some(0.0));
    assert_eq!(keyed("end", 5.0, 0.0, 100.0, 1.0), Some(100.0));
    assert_eq!(keyed("a", 5.0, 0.0, 100.0, 1.0), None);
}
