use super::*;

#[test]
fn only_a_name_match_marks_the_name() {
    let project = Project {
        name: "中文".into(),
        path: "foo".into(),
        branch: None,
        opened: "now".into(),
        pinned: false,
    };
    assert_eq!(fit("oo", &project).map(|(_, hits)| hits), Some(Vec::new()));
    assert!(fit("zz", &project).is_none());
}

#[test]
fn pinned_projects_lead_and_keep_their_places() {
    let project = |name: &str, pinned: bool| Project {
        name: name.to_string().into(),
        path: "~".into(),
        branch: None,
        opened: "now".into(),
        pinned,
    };
    let projects = [project("a", false), project("b", true), project("c", false)];
    let order: Vec<usize> = ordered(&projects).into_iter().map(|(ix, _)| ix).collect();
    assert_eq!(order, [1, 0, 2]);
}
