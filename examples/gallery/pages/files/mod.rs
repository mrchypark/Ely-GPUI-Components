use gpui::prelude::FluentBuilder as _;

use ely_gpui_component::{
    files::{FileGrid, FileIcon, FileItem},
    lists::{DirEntry, DirectoryListing, SelectableList},
    theme::{ActiveTheme, IconSize, TextSize},
};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, Window,
    div, px,
};
use jiff::{Timestamp, ToSpan};

mod actions;
mod explorer;
mod find;
#[cfg(all(test, feature = "test-support", not(target_family = "wasm")))]
mod touch_tests;
mod transfers;

use super::Page;
use crate::{
    probe::probe,
    step::Step,
    ui::{change, keep, section},
};

pub const PAGE: Page = Page {
    number: 27,
    slug: "files",
    title: "Files",
    summary: "Files and folders: their icons, rows and tiles, and the listings, searches and transfers around them.",
    render,
    script: SCRIPT,
};

const SCRIPT: &[Step] = &[
    Step::DownAt("files-grid", 150.0, 40.0),
    Step::UpAt("files-grid", 150.0, 40.0),
    Step::Key("right"),
    Step::Key("down"),
    Step::Wait(200),
    Step::Shot("grid-picked"),
    Step::DownAt("files-explorer", 515.0, 14.0),
    Step::UpAt("files-explorer", 515.0, 14.0),
    Step::Wait(300),
    Step::Shot("explorer-icons"),
    Step::DownAt("files-explorer", 48.0, 120.0),
    Step::UpAt("files-explorer", 48.0, 120.0),
    Step::Key("enter"),
    Step::Wait(300),
    Step::Shot("explorer-into"),
    Step::RightAt("files-context", 150.0, 50.0),
    Step::Wait(300),
    Step::Shot("context-menu"),
    Step::Key("escape"),
    Step::DownAt("files-rename", 80.0, 14.0),
    Step::UpAt("files-rename", 80.0, 14.0),
    Step::Type("Brief notes.md"),
    Step::Key("enter"),
    Step::Wait(300),
    Step::Shot("renamed"),
    Step::DownAt("files-quick", 150.0, 40.0),
    Step::UpAt("files-quick", 150.0, 40.0),
    Step::Key("space"),
    Step::Wait(300),
    Step::Shot("quick-look"),
    Step::Key("escape"),
    Step::DownAt("files-search", 100.0, 16.0),
    Step::UpAt("files-search", 100.0, 16.0),
    Step::Type("a"),
    Step::Wait(300),
    Step::Shot("searched"),
];

/// A project's entries, changed a few hours to a few weeks ago.
pub fn entries() -> Vec<DirEntry> {
    let now = Timestamp::now();
    let ago = |hours: i64| now - hours.hours();
    vec![
        DirEntry::folder("Design", ago(3)),
        DirEntry::folder("Recordings", ago(30)),
        DirEntry::file("Brief.md", 4_800, ago(2)),
        DirEntry::file("Budget.csv", 18_300, ago(26)),
        DirEntry::file("Atrium.jpg", 2_140_000, ago(50)),
        DirEntry::file("Walkthrough.mov", 184_000_000, ago(120)),
        DirEntry::file("main.rs", 9_200, ago(1)),
        DirEntry::file("Archive.zip", 48_000_000, ago(700)),
        DirEntry::file(
            "Quarterly review with the design team, final.pdf",
            820_000,
            ago(6),
        ),
    ]
}

fn icons(cx: &mut App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let (small, muted) = (theme.text_size(TextSize::Xs), theme.colors.fg_muted);
    let names = [
        "Brief.md",
        "main.rs",
        "build.sh",
        "config.toml",
        "data.json",
        "Atrium.jpg",
        "Theme.mp3",
        "Clip.mov",
        "Archive.zip",
        "Budget.csv",
        "Inter.ttf",
        "Makefile",
    ];
    let tile = |icon: FileIcon, name: &'static str| {
        div()
            .w(px(88.))
            .flex()
            .flex_col()
            .items_center()
            .gap_1p5()
            .child(icon.size(IconSize::Xl))
            .child(div().text_size(small).text_color(muted).child(name))
    };
    section(
        "FileIcon",
        "A file's icon by its name's extension: code, text, pictures, sound, video, archives, tables, fonts and scripts; a folder shut or open.",
        cx,
    )
    .child(probe(
        "files-icons",
        div()
            .w(px(640.))
            .flex()
            .flex_wrap()
            .gap_y_4()
            .child(tile(FileIcon::folder(false), "Folder"))
            .child(tile(FileIcon::folder(true), "Open"))
            .children(names.map(|name| tile(FileIcon::file(name), name))),
    ))
}

fn items(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let picked = keep("files-items", || SharedString::from("Brief.md"), window, cx);
    let now = picked.read(cx).clone();
    let pick = picked.clone();
    let list =
        entries()
            .into_iter()
            .take(5)
            .fold(SelectableList::new("files-items"), |list, entry| {
                let key = entry.name().clone();
                list.row(
                    key.clone(),
                    FileItem::new(SharedString::from(format!("files-item-{key}")), entry).into(),
                )
            });
    section(
        "FileItem",
        "A file or folder as a row: its icon and name over its size and when it changed.",
        cx,
    )
    .child(probe(
        "files-items",
        div()
            .w(px(420.))
            .child(list.selected([now]).on_change(move |keys, _, cx| {
                if let Some(key) = keys.first() {
                    change(&pick, cx, |now| *now = key.clone())
                }
            })),
    ))
}

fn listing(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let touch = cx.theme().touch_mode;
    let opened = keep(
        "files-listing-opened",
        || (0usize, SharedString::default()),
        window,
        cx,
    );
    let (count, name) = opened.read(cx).clone();
    section(
        "FileList",
        "Select an entry, then tap Open on touch. Desktop also opens with Enter or double press. This fixture records the callback only.",
        cx,
    )
    .child(probe(
        "files-listing",
        div().when(touch, |view| view.w_full().min_w_0()).when(!touch, |view| view.w(px(640.))).h(px(360.)).child(
            DirectoryListing::new("files-listing", ["Home", "Projects", "Atrium"], entries())
                .on_open(move |entry, _, cx| change(&opened, cx, |opened| {
                    opened.0 += 1;
                    opened.1 = entry.name().clone();
                }))
                .on_climb(|level, _, _| log::info!("gallery: climb to {level}")),
        ),
    ))
    .children((count > 0).then(|| div().debug_selector(|| "files-listing-result".into())
        .child(format!("Open {name}: local callback {count}; no filesystem operation."))))
}

fn grid(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let touch = cx.theme().touch_mode;
    let opened = keep(
        "files-grid-opened",
        || (0usize, None::<SharedString>),
        window,
        cx,
    );
    let (open_count, opened_name) = opened.read(cx).clone();
    let picked = keep("files-grid", || None::<SharedString>, window, cx);
    let now = picked.read(cx).clone();
    let pick = picked.clone();
    let grid = FileGrid::new("files-grid", entries())
        .on_select(move |name, _, cx| change(&pick, cx, |now| *now = Some(name.clone())))
        .on_open(move |entry, _, cx| {
            log::info!("gallery: open {}", entry.name());
            change(&opened, cx, |(count, name)| {
                *count += 1;
                *name = Some(entry.name().clone());
            });
        });
    let grid = match now {
        Some(name) => grid.selected(name),
        None => grid,
    };
    section(
        "FileGrid",
        if touch { "Tap once to select a file or folder, then tap Open. This gallery records the chosen entry locally." } else { "Files and folders as tiles. A press picks one and a double press opens it; with focus the arrows move by tile and by row, and Enter opens." },
        cx,
    )
    .child(probe("files-grid", div().when(touch, |view| view.w_full().min_w_0()).when(!touch, |view| view.w(px(640.))).child(grid)))
    .children(touch.then_some(opened_name).flatten().map(|name| {
        let caption = format!("Open {name}: local callback {open_count}; no filesystem operation.");
        div()
            .debug_selector(move || format!("gallery-file-opened-{name}"))
            .child(caption)
    }))
}

fn render(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> AnyElement {
    div()
        .child(icons(cx))
        .child(items(window, cx))
        .child(listing(window, cx))
        .child(grid(window, cx))
        .child(explorer::explorer(window, cx))
        .child(explorer::tree(cx))
        .child(actions::context_menu(window, cx, bounds))
        .child(actions::rename(window, cx))
        .child(actions::drop_files(window, cx))
        .child(find::previews(window, cx, bounds))
        .child(find::recent(cx))
        .child(find::favorites(window, cx))
        .child(find::search(window, cx))
        .child(find::duplicates(window, cx))
        .child(find::archive(cx))
        .child(find::hex(window, cx))
        .child(transfers::operations(window, cx))
        .child(transfers::queue(window, cx))
        .child(transfers::downloads(window, cx))
        .child(transfers::storage(cx))
        .into_any_element()
}
