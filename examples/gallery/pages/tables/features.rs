use ely_gpui_component::{
    data_display::{DescriptionList, Tone},
    tables::{Cell, Column, DataTable, Row},
    theme::ActiveTheme,
    typography::Caption,
};
use gpui::{App, IntoElement, ParentElement, SharedString, Styled, Window, div, px, rems};

use super::core::{order_columns, order_rows};
use crate::{
    probe::probe,
    ui::{keep, section, set},
};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn wide(cx: &mut App) -> impl IntoElement + use<> {
    let touch = cx.theme().touch_mode;
    let frame = if touch {
        div().w_full().min_w_0()
    } else {
        div().w(px(620.))
    };
    let columns = std::iter::once(Column::new("team", "Team").width(rems(10.)).pinned()).chain(
        MONTHS
            .iter()
            .map(|month| Column::new(*month, *month).width(rems(5.5)).end()),
    );
    let rows = ["Design", "Platform", "Apps", "Growth", "Support"]
        .iter()
        .enumerate()
        .map(|(ix, team)| {
            Row::new(
                *team,
                std::iter::once(Cell::from(*team)).chain(
                    (0..12).map(|month| Cell::Number(((ix * 7 + month * 13) % 40 + 10) as f64)),
                ),
            )
        });
    section(
        "ColumnPinning / ColumnResizer / ColumnReorder",
        if touch { "Team stays pinned. Open Adjust columns, choose a column with Previous/Next, then move it left/right or change its width. Reset width restores its declared size." } else { "Team stays put while the months scroll sideways. Drag a header's right edge to resize it; drag a header onto another to move it there." },
        cx,
    )
    .child(frame.child(DataTable::new("wide", columns).rows(rows.collect::<Vec<_>>()).w_full()))
}

/// Tasks the demo edits in place: key, name, owner, state.
fn tasks() -> Vec<(SharedString, SharedString, &'static str, &'static str)> {
    [
        ("t1", "Draft the launch notes", "Ada Lovelace", "Doing"),
        ("t2", "Review the type scale", "Grace Hopper", "Done"),
        ("t3", "Record the demo", "Alan Turing", "Next"),
    ]
    .map(|(key, name, owner, state)| (key.into(), name.into(), owner, state))
    .to_vec()
}

pub fn rows(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let kept = keep("tasks", tasks, window, cx);
    let (now, store) = (kept.read(cx).clone(), kept.clone());
    let rows: Vec<Row> = now
        .iter()
        .map(|(key, name, owner, state)| {
            let tone = match *state {
                "Done" => Tone::Success,
                "Doing" => Tone::Info,
                _ => Tone::Neutral,
            };
            Row::new(
                key.clone(),
                [
                    Cell::Text(name.clone()),
                    Cell::Person((*owner).into()),
                    Cell::Tag((*state).into(), tone),
                ],
            )
        })
        .collect();
    let about = now.clone();
    section(
        "RowExpansion / InlineRowEdit",
        "The chevron opens a row's detail beneath it. Double press a task's name to edit it; Enter keeps the change, Escape drops it.",
        cx,
    )
    .child(probe(
        "tasks",
        div().w(px(640.)).child(
            DataTable::new(
                "tasks",
                [Column::new("name", "Task").editable(), Column::new("owner", "Owner").width(rems(12.)), Column::new("state", "State").width(rems(6.))],
            )
            .rows(rows)
            .detail(move |key, _, _| {
                let (_, name, owner, state) = about.iter().find(|task| task.0 == *key).expect("a listed task").clone();
                DescriptionList::new()
                    .item("Task", name)
                    .item("Owner", owner)
                    .item("State", state)
                    .into_any_element()
            })
            .on_edit(move |key, _, text, _, cx| {
                let mut next = store.read(cx).clone();
                if let Some(task) = next.iter_mut().find(|task| task.0 == *key) {
                    task.1 = text.clone();
                }
                set(&store, next, cx)
            }),
        ),
    ))
}

pub fn grouped(cx: &mut App) -> impl IntoElement + use<> {
    section(
        "RowGrouping",
        "Orders gathered by region, each group with its count; a press on a group folds it.",
        cx,
    )
    .child(probe(
        "grouped",
        div().w(px(900.)).child(
            DataTable::new("grouped", order_columns())
                .rows(order_rows().into_iter().take(9).collect::<Vec<_>>())
                .group_by("region"),
        ),
    ))
    .child(Caption::new(
        "Groups keep the order their first rows come in; sort to reorder them.",
    ))
}

#[cfg(test)]
mod column_touch_tests {
    use super::*;
    use ely_gpui_component::theme::Theme;
    use gpui::{Context, Modifiers, Render, TestAppContext, VisualTestContext, size};

    struct Specimen;
    impl Render for Specimen {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().w_full().child(super::wide(cx))
        }
    }
    fn settle(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    fn tap(selector: &'static str, cx: &mut VisualTestContext) {
        let bounds = cx.debug_bounds(selector).expect(selector);
        cx.simulate_click(bounds.center(), Modifiers::none());
        settle(cx);
    }
    #[gpui::test]
    fn real_column_specimen_has_contained_touch_controls_at_both_phone_widths(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (_, cx) = cx.add_window_view(|window, _| {
            window.set_rem_size(px(16.));
            Specimen
        });
        cx.simulate_resize(size(px(390.), px(844.)));
        settle(cx);
        tap("table-columns-toggle", cx);
        for width in [390., 320.] {
            cx.simulate_resize(size(px(width), px(844.)));
            settle(cx);
            for selector in [
                "table-columns-toggle",
                "table-column-Previous",
                "table-column-Next",
                "table-column-Move left",
                "table-column-Move right",
                "table-column-Narrower",
                "table-column-Wider",
                "table-column-Reset width",
            ] {
                let bounds = cx.debug_bounds(selector).expect(selector);
                assert!(
                    bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
                    "{selector}: {bounds:?}"
                );
                assert!(
                    bounds.left() >= px(0.)
                        && bounds.right() <= px(width)
                        && bounds.top() >= px(0.)
                        && bounds.bottom() <= px(844.),
                    "{selector}: {bounds:?}"
                );
            }
            let before = cx.debug_bounds("table-head team").unwrap();
            tap("table-column-Wider", cx);
            assert_eq!(
                cx.debug_bounds("table-head team").unwrap().size.width,
                before.size.width + px(24.)
            );
            tap("table-column-Reset width", cx);
            assert_eq!(cx.debug_bounds("table-head team").unwrap(), before);
        }
    }
}
