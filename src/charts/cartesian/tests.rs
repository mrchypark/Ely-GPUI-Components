use super::{Points, scatter::locate};

#[test]
fn a_flat_index_finds_its_cloud() {
    let clouds = [
        Points::new("a", [(0.0, 0.0), (1.0, 1.0)]),
        Points::new("b", [(2.0, 2.0)]),
    ];
    assert_eq!(locate(&clouds, 1), (0, 1));
    assert_eq!(locate(&clouds, 2), (1, 0));
}
