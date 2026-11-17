use gpui::{Bounds, Pixels};

use super::super::model::{Kind, Menu};

/// The menu open at `depth`, following the marked rows above it; `None` once those rows changed.
pub(in crate::menus) fn at_depth<'a>(
    menu: &'a Menu,
    levels: &[Level],
    depth: usize,
) -> Option<&'a Menu> {
    levels.get(..depth)?.iter().try_fold(menu, |menu, level| {
        match level
            .at
            .and_then(|ix| menu.item_at(ix))
            .map(|item| &item.kind)
        {
            Some(Kind::Sub(sub)) => Some(sub),
            _ => None,
        }
    })
}

/// Fits open levels to the menu as it is now: a mark on a row gone or disabled clears, and levels under a row that no longer opens them close.
pub(in crate::menus) fn settle(menu: &Menu, levels: &mut Vec<Level>) {
    let mut here = menu;
    for depth in 0..levels.len() {
        let at = levels[depth]
            .at
            .filter(|&ix| here.item_at(ix).is_some_and(|item| !item.disabled));
        levels[depth].at = at;
        match at.and_then(|ix| here.item_at(ix)).map(|item| &item.kind) {
            Some(Kind::Sub(sub)) if depth + 1 < levels.len() => here = sub,
            _ => {
                levels.truncate(depth + 1);
                return;
            }
        }
    }
}

/// One open level: its marked row, that row's box and its panel's box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::menus) struct Level {
    pub(in crate::menus) at: Option<usize>,
    pub(in crate::menus) row: Bounds<Pixels>,
    pub(in crate::menus) panel: Bounds<Pixels>,
}
