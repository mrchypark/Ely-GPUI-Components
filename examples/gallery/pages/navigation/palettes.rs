use gpui::prelude::FluentBuilder as _;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::Choice,
    navigation::{Command, CommandPalette, QuickLauncher, QuickOpen, QuickSwitcher, SearchPalette},
    primitives::IconName,
    theme::ActiveTheme,
};
use gpui::{
    App, Bounds, Entity, InteractiveElement, IntoElement, ParentElement, Pixels, SharedString,
    Window,
};

use crate::{
    probe::{Opened, probe},
    ui::{keep, native_only, row, section, set, specimen},
};

/// Moves `value` to the front of a recent list, keeping `size` entries.
fn remember(list: &Entity<Vec<SharedString>>, value: &SharedString, size: usize, cx: &mut App) {
    list.update(cx, |list, cx| {
        list.retain(|kept| kept != value);
        list.insert(0, value.clone());
        list.truncate(size);
        cx.notify();
    });
}

fn commands() -> Vec<(&'static str, Vec<Command>)> {
    let command =
        |value: &'static str, label: &'static str, icon| Command::new(value, label).icon(icon);
    vec![
        (
            "File",
            vec![
                command("new", "New file", IconName::FilePlus).keys("secondary-n"),
                command("open", "Open…", IconName::FolderOpen).keys("secondary-o"),
                command("save", "Save", IconName::Save).keys("secondary-s"),
                command("share", "Share…", IconName::Share2),
            ],
        ),
        (
            "View",
            vec![
                command("sidebar", "Toggle sidebar", IconName::PanelLeft).keys("secondary-b"),
                command("terminal", "Toggle terminal", IconName::Terminal).keys("ctrl-`"),
                command("zoom-in", "Zoom in", IconName::ZoomIn).keys("secondary-="),
                command("zoom-out", "Zoom out", IconName::ZoomOut).keys("secondary--"),
            ],
        ),
        (
            "Theme",
            vec![
                command("light", "Light theme", IconName::Sun),
                command("dark", "Dark theme", IconName::Moon),
                command("system", "Match the system", IconName::Monitor),
            ],
        ),
        (
            "Help",
            vec![
                command("keys", "Keyboard shortcuts", IconName::Keyboard).keys("secondary-/"),
                command("bug", "Report a bug", IconName::Bug),
                command("new-in", "What's new", IconName::Sparkles),
            ],
        ),
    ]
}

pub fn command_palette(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<Bounds<Pixels>>,
) -> impl IntoElement + use<> {
    let open = keep("commands-open", || false, window, cx);
    let recent = keep(
        "commands-recent",
        || vec!["sidebar".into(), "dark".into()],
        window,
        cx,
    );
    let ran = keep("commands-ran", || None::<SharedString>, window, cx);
    let caption = ran
        .read(cx)
        .clone()
        .map_or("apps open it with ⌘K".into(), |value| {
            format!("ran {value}")
        });
    let (opener, closer) = (open.clone(), open.clone());
    let palette = (*open.read(cx)).then(|| {
        let recents = recent.read(cx).clone();
        commands()
            .into_iter()
            .fold(
                CommandPalette::new("commands", move |_, cx| set(&closer, false, cx)),
                |palette, (title, commands)| palette.group(title, commands),
            )
            .recent(recents)
            .when_some(bounds, |palette, area| palette.usable_bounds(area))
            .on_run(move |value, _, cx| {
                remember(&recent, value, 3, cx);
                set(&ran, Some(value.clone()), cx);
            })
    });
    section(
        "CommandPalette",
        "Every command in one place. Recent ones lead; typing ranks fuzzy matches and marks the letters that matched.",
        cx,
    )
    .child(specimen(
        caption,
        probe(
            "commands",
            Button::new("commands-button", "Command palette")
                .icon(IconName::Command)
                .on_click(move |_, _, cx| set(&opener, true, cx)),
        ),
        cx,
    ))
    .children(palette)
}

const FILES: [&str; 24] = [
    "Cargo.toml",
    "README.md",
    "AGENTS.md",
    "src/lib.rs",
    "src/theme/mod.rs",
    "src/theme/palette.rs",
    "src/theme/tokens.rs",
    "src/motion/mod.rs",
    "src/motion/slide.rs",
    "src/primitives/icon.rs",
    "src/primitives/focus.rs",
    "src/buttons/button.rs",
    "src/buttons/segmented.rs",
    "src/forms/select.rs",
    "src/forms/combobox.rs",
    "src/forms/text/mod.rs",
    "src/forms/date/picker.rs",
    "src/navigation/tabs.rs",
    "src/navigation/menu.rs",
    "src/navigation/palette.rs",
    "src/navigation/toc.rs",
    "examples/gallery/main.rs",
    "examples/gallery/pages/navigation/palettes.rs",
    "frontend/package.json",
];

pub fn quick_open(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> impl IntoElement + use<> {
    let open = keep("files-open", || false, window, cx);
    let recent = keep(
        "files-recent",
        || vec!["src/theme/tokens.rs".into(), "AGENTS.md".into()],
        window,
        cx,
    );
    let opened = keep("files-opened", || None::<SharedString>, window, cx);
    let caption = opened
        .read(cx)
        .clone()
        .map_or("fuzzy over the whole path".into(), |path| {
            format!("opened {path}")
        });
    let (opener, closer) = (open.clone(), open.clone());
    let palette = (*open.read(cx)).then(|| {
        QuickOpen::new("files", FILES, move |_, cx| set(&closer, false, cx))
            .when_some(bounds, |palette, area| palette.usable_bounds(area))
            .recent(recent.read(cx).clone())
            .on_open(move |path, _, cx| {
                remember(&recent, path, 4, cx);
                set(&opened, Some(path.clone()), cx);
            })
    });
    section(
        "QuickOpen",
        "Files by path. The name leads and its folder follows; recent files show before you type.",
        cx,
    )
    .child(specimen(
        caption,
        probe(
            "files",
            Button::new("files-button", "Go to file")
                .debug_selector(|| "gallery-quick-open".into())
                .icon(IconName::File)
                .on_click(move |_, _, cx| set(&opener, true, cx)),
        ),
        cx,
    ))
    .children(palette)
}

fn places() -> Vec<Choice> {
    vec![
        Choice::new("roadmap", "Roadmap 2027")
            .icon(IconName::FileText)
            .note("Document"),
        Choice::new("standup", "Design standup")
            .icon(IconName::MessageSquare)
            .note("Session"),
        Choice::new("ely", "Ely GPUI Component")
            .icon(IconName::Box)
            .note("Project"),
        Choice::new("brand", "Brand guidelines")
            .icon(IconName::FileText)
            .note("Document"),
        Choice::new("research", "User research")
            .icon(IconName::MessageSquare)
            .note("Session"),
        Choice::new("site", "ely.dev")
            .icon(IconName::Globe)
            .note("Project"),
    ]
}

pub fn quick_switcher(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> impl IntoElement + use<> {
    let open = keep("switch-open", || false, window, cx);
    let order = keep(
        "switch-order",
        || {
            places()
                .into_iter()
                .map(|place| place.value)
                .collect::<Vec<_>>()
        },
        window,
        cx,
    );
    let (opener, closer) = (open.clone(), open.clone());
    let current = order.read(cx)[0].clone();
    let palette = (*open.read(cx)).then(|| {
        let all = places();
        let items = order.read(cx).iter().map(|value| {
            all.iter()
                .find(|place| place.value == *value)
                .expect("every kept place exists")
                .clone()
        });
        QuickSwitcher::new("switcher", items, move |_, cx| set(&closer, false, cx))
            .when_some(bounds, |palette, area| palette.usable_bounds(area))
            .on_switch(move |value, _, cx| remember(&order, value, 6, cx))
    });
    section(
        "QuickSwitcher",
        "Documents, sessions and projects, most recent first. It opens on the one before, so Enter goes back.",
        cx,
    )
    .child(specimen(
        format!("in {current}"),
        probe(
            "switcher",
            Button::new("switcher-button", "Switch")
                .debug_selector(|| "gallery-quick-switcher".into())
                .icon(IconName::Layers)
                .on_click(move |_, _, cx| set(&opener, true, cx)),
        ),
        cx,
    ))
    .children(palette)
}

type Found = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    IconName,
);

const FOUND: [Found; 12] = [
    (
        "Components",
        "palette",
        "Command palette",
        "Navigation · every command in one place",
        IconName::Command,
    ),
    (
        "Components",
        "date",
        "Date picker",
        "Forms · a calendar under a field",
        IconName::Calendar,
    ),
    (
        "Components",
        "color",
        "Color picker",
        "Forms · hue, saturation and alpha",
        IconName::Pipette,
    ),
    (
        "Components",
        "tabs",
        "Tabs",
        "Navigation · a line slides to the chosen tab",
        IconName::PanelLeft,
    ),
    (
        "Components",
        "slider",
        "Slider",
        "Forms · a thumb on a track, with steps",
        IconName::SlidersHorizontal,
    ),
    (
        "Guides",
        "tokens",
        "Theme and tokens",
        "Colors, sizes and radii come from the theme",
        IconName::Palette,
    ),
    (
        "Guides",
        "motion",
        "Motion",
        "Durations honor reduced motion; springs stay in animators",
        IconName::Sparkles,
    ),
    (
        "Guides",
        "focus",
        "Focus",
        "Tab and Shift-Tab walk every stop in order",
        IconName::Keyboard,
    ),
    (
        "Guides",
        "capture",
        "Captures",
        "The gallery photographs its own windows",
        IconName::Image,
    ),
    (
        "People",
        "ada",
        "Ada Lovelace",
        "Design · London",
        IconName::User,
    ),
    (
        "People",
        "grace",
        "Grace Hopper",
        "Engineering · Arlington",
        IconName::User,
    ),
    (
        "People",
        "alan",
        "Alan Kay",
        "Research · Los Angeles",
        IconName::User,
    ),
];

/// Case-insensitive search over titles and notes; the empty query suggests a few.
fn search(query: &str) -> Vec<(SharedString, Vec<Choice>)> {
    let query = query.to_lowercase();
    let result =
        |(_, value, label, note, icon): &Found| Choice::new(*value, *label).icon(*icon).note(*note);
    if query.is_empty() {
        let suggested = FOUND
            .iter()
            .filter(|found| ["palette", "tokens", "motion", "ada"].contains(&found.1));
        return vec![("Suggested".into(), suggested.map(result).collect())];
    }
    ["Components", "Guides", "People"]
        .into_iter()
        .map(|group| {
            let hits = FOUND.iter().filter(|found| {
                found.0 == group
                    && (found.2.to_lowercase().contains(&query)
                        || found.3.to_lowercase().contains(&query))
            });
            (group.into(), hits.map(result).collect())
        })
        .collect()
}

pub fn search_palette(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<Bounds<Pixels>>,
) -> impl IntoElement + use<> {
    let inline = cfg!(any(target_os = "ios", target_os = "android")) || cx.theme().touch_mode;
    let open = keep("search-open", || false, window, cx);
    let went = keep("search-went", || None::<SharedString>, window, cx);
    let result = went.read(cx).clone();
    let caption = went
        .read(cx)
        .clone()
        .map_or("results grouped by where they were found".into(), |value| {
            format!("opened {value}")
        });
    let (opener, closer, launched) = (open.clone(), open.clone(), went.clone());
    let inline_opener = open.clone();
    let palette = (*open.read(cx)).then(|| {
        SearchPalette::new("search", search, move |_, cx| set(&closer, false, cx))
            .when_some(bounds, |palette, area| palette.usable_bounds(area))
            .on_open(move |value, _, cx| set(&went, Some(value.clone()), cx))
    });
    section(
        "SearchPalette / SpotlightSearch / QuickLauncher",
        if inline { "SearchPalette searches in this surface. The launcher entry uses the same real inline search; it does not open an OS QuickLauncher window." } else { "One field searches everywhere and marks each match. As a launcher it opens alone, above every window, and closes when you look away." },
        cx,
    )
    .child(
        row()
            .child(specimen(
                caption,
                probe(
                    "search",
                    Button::new("search-button", "Search everything")
                        .debug_selector(|| "gallery-search-open".into())
                        .icon(IconName::Search)
                        .on_click(move |_, _, cx| set(&opener, true, cx)),
                ),
                cx,
            ).debug_selector(move || format!("gallery-search-result-{}", result.as_deref().unwrap_or("none"))))
            .child(native_only(specimen(
                if inline { "inline search; native QuickLauncher unavailable here" } else { "a window above all others" },
                probe(
                    "launcher",
                    Button::new("launcher-button", if inline { "Open inline search" } else { "Open the launcher" })
                        .debug_selector(|| "gallery-launcher-open".into())
                        .variant(ButtonVariant::Ghost)
                        .icon(IconName::Rocket)
                        .on_click(move |_, _, cx| {
                            if inline {
                                set(&inline_opener, true, cx);
                                return;
                            }
                            let went = launched.clone();
                            let opened = QuickLauncher::open(
                                search,
                                move |value, _, cx| set(&went, Some(value.clone()), cx),
                                cx,
                            );
                            match opened {
                                Ok(handle) => Opened::insert("launcher", handle.into(), cx),
                                Err(error) => log::error!("gallery: launcher: {error:#}"),
                            }
                        }),
                ),
                cx,
            ))),
    )
    .children(palette)
}
