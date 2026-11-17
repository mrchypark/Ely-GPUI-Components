use std::rc::Rc;

use emojis::Group;
use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use super::picks::{Grid, Pick, finder, grid, rows};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::Emoji,
};

/// Glyphs across an emoji or icon grid.
const COLUMNS: usize = 8;

type OnIcon = Rc<dyn Fn(IconName, &mut Window, &mut App)>;
type OnEmoji = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// A searchable grid of the library's icons.
#[derive(IntoElement)]
pub struct IconPicker {
    id: ElementId,
    selected: Option<IconName>,
    on_change: Option<OnIcon>,
}

impl IconPicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            on_change: None,
        }
    }

    pub fn selected(mut self, icon: IconName) -> Self {
        self.selected = Some(icon);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(IconName, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for IconPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let finder = finder(&self.id, "Search icons", window, cx);
        let query = finder.read(cx).query.clone();
        let found: Rc<Vec<IconName>> = Rc::new(
            IconName::ALL
                .iter()
                .copied()
                .filter(|icon| icon.name().replace('-', " ").contains(&query))
                .collect(),
        );
        let (painted, picked) = (found.clone(), found.clone());
        let (id, on_change) = (self.id.clone(), self.on_change);
        let pick: Pick = Rc::new(move |ix, window, cx| {
            let icon = picked[ix];
            log::info!("icon picker {id:?}: {}", icon.name());
            if let Some(on_change) = &on_change {
                on_change(icon, window, cx);
            }
        });
        let fg = cx.theme().colors.fg;
        grid(
            Grid {
                id: self.id,
                names: Rc::new(found.iter().map(|icon| icon.name().into()).collect()),
                rows: Rc::new(rows([(None, found.len())], COLUMNS)),
                selected: self
                    .selected
                    .and_then(|icon| found.iter().position(|f| *f == icon)),
                draw: Rc::new(move |ix, _| {
                    Icon::new(painted[ix])
                        .size(IconSize::Md)
                        .color(fg)
                        .into_any_element()
                }),
                pick,
                none: "No icons match",
                columns: COLUMNS,
                cell: cx.theme().control_height(ControlSize::Lg),
            },
            &finder,
            window,
            cx,
        )
    }
}

/// Emoji whose name or shortcode holds `query`, a lowercase string; all for an empty one.
pub(crate) fn emoji_found(query: &str) -> Vec<&'static emojis::Emoji> {
    emojis::iter()
        .filter(|emoji| {
            query.is_empty()
                || emoji.name().to_lowercase().contains(query)
                || emoji.shortcodes().any(|code| code.contains(query))
        })
        .collect()
}

fn title(group: Group) -> &'static str {
    match group {
        Group::SmileysAndEmotion => "Smileys & Emotion",
        Group::PeopleAndBody => "People & Body",
        Group::AnimalsAndNature => "Animals & Nature",
        Group::FoodAndDrink => "Food & Drink",
        Group::TravelAndPlaces => "Travel & Places",
        Group::Activities => "Activities",
        Group::Objects => "Objects",
        Group::Symbols => "Symbols",
        Group::Flags => "Flags",
    }
}

/// A searchable grid of emoji, grouped as Unicode groups them.
#[derive(IntoElement)]
pub struct EmojiPicker {
    id: ElementId,
    selected: Option<SharedString>,
    on_change: Option<OnEmoji>,
}

impl EmojiPicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            on_change: None,
        }
    }

    pub fn selected(mut self, emoji: impl Into<SharedString>) -> Self {
        self.selected = Some(emoji.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EmojiPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let finder = finder(&self.id, "Search emoji", window, cx);
        let query = finder.read(cx).query.clone();
        let found = emoji_found(&query);
        let layout = if query.is_empty() {
            rows(
                Group::iter().map(|group| (Some(title(group).into()), group.emojis().count())),
                COLUMNS,
            )
        } else {
            rows([(None, found.len())], COLUMNS)
        };
        let found = Rc::new(found);
        let (painted, picked) = (found.clone(), found.clone());
        let (id, on_change) = (self.id.clone(), self.on_change);
        let pick: Pick = Rc::new(move |ix, window, cx| {
            let emoji = picked[ix];
            log::info!("emoji picker {id:?}: {}", emoji.name());
            if let Some(on_change) = &on_change {
                on_change(emoji.as_str(), window, cx);
            }
        });
        grid(
            Grid {
                id: self.id,
                names: Rc::new(found.iter().map(|emoji| emoji.name().into()).collect()),
                rows: Rc::new(layout),
                selected: self.selected.and_then(|glyph| {
                    found
                        .iter()
                        .position(|emoji| emoji.as_str() == glyph.as_ref())
                }),
                draw: Rc::new(move |ix, _| {
                    Emoji::new(painted[ix].as_str())
                        .size(TextSize::Lg)
                        .into_any_element()
                }),
                pick,
                none: "No emoji match",
                columns: COLUMNS,
                cell: cx.theme().control_height(ControlSize::Lg),
            },
            &finder,
            window,
            cx,
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{
        Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
        Window, div, px,
    };

    use super::{EmojiPicker, IconPicker, emoji_found};
    use crate::theme::Theme;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Picker {
        Emoji,
        Icon,
    }

    struct PickerProbe {
        picker: Picker,
        width: f32,
    }

    impl Render for PickerProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let picker = match self.picker {
                Picker::Emoji => EmojiPicker::new("emoji-touch").into_any_element(),
                Picker::Icon => IconPicker::new("icon-touch").into_any_element(),
            };
            div()
                .debug_selector(|| "picker-host".into())
                .w(px(self.width))
                .child(picker)
        }
    }

    #[test]
    fn emoji_are_found_whatever_the_case_of_their_names() {
        let glyphs = |query| {
            emoji_found(query)
                .into_iter()
                .map(|emoji| emoji.as_str())
                .collect::<Vec<_>>()
        };
        assert!(glyphs("united states").contains(&"🇺🇸"));
        assert!(glyphs("sparkles").contains(&"✨"));
        assert_eq!(glyphs("").len(), emojis::iter().count());
    }

    #[gpui::test]
    fn touch_pickers_fit_and_use_five_48px_columns_in_240px(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        for picker in [Picker::Emoji, Picker::Icon] {
            let (_view, cx) = cx.add_window_view(|_, _| PickerProbe {
                picker,
                width: 240.,
            });
            cx.simulate_resize(gpui::size(px(390.), px(844.)));
            cx.update(|window, _| window.activate_window());
            cx.run_until_parked();

            let host = cx.debug_bounds("picker-host").expect("picker host");
            let grid = cx.debug_bounds("glyph-grid").expect("glyph grid");
            assert!(grid.size.width <= host.size.width, "{picker:?}: {grid:?}");
            let cells: Vec<_> = [
                "glyph-cell-0",
                "glyph-cell-1",
                "glyph-cell-2",
                "glyph-cell-3",
                "glyph-cell-4",
                "glyph-cell-5",
            ]
            .into_iter()
            .map(|name| {
                cx.debug_bounds(name)
                    .unwrap_or_else(|| panic!("{picker:?}: {name}"))
            })
            .collect();
            assert!(cells.iter().all(|cell| cell.size.width >= px(48.)));
            assert!(cells.iter().all(|cell| cell.size.height >= px(48.)));
            assert!(cells[5].top() > cells[4].top(), "{picker:?}: {cells:?}");
        }
    }

    #[gpui::test]
    fn resized_icon_grid_keeps_the_cursor_in_view(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, _| PickerProbe {
            picker: Picker::Icon,
            width: 480.,
        });
        cx.simulate_resize(gpui::size(px(800.), px(844.)));
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        cx.update(|window, cx| window.focus_next(cx));
        cx.simulate_keystrokes("down");
        for _ in 0..12 {
            cx.simulate_keystrokes("down");
        }
        cx.run_until_parked();
        assert!(super::IconName::ALL.len() > 96);
        assert!(cx.debug_bounds("glyph-cursor").is_some());

        view.update(cx, |view, cx| {
            view.width = 240.;
            cx.notify();
        });
        cx.run_until_parked();
        let grid = cx.debug_bounds("glyph-grid").expect("glyph grid");
        let cursor = cx
            .debug_bounds("glyph-cursor")
            .expect("cursor row stays visible");
        assert!(cursor.top() >= grid.top() && cursor.bottom() <= grid.bottom());
    }

    #[gpui::test]
    fn unmatched_pickers_survive_a_column_resize(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        for picker in [Picker::Emoji, Picker::Icon] {
            let (view, cx) = cx.add_window_view(|_, _| PickerProbe {
                picker,
                width: 480.,
            });
            cx.simulate_resize(gpui::size(px(800.), px(844.)));
            cx.update(|window, _| window.activate_window());
            cx.run_until_parked();
            cx.update(|window, cx| window.focus_next(cx));
            cx.simulate_input("no-such-glyph-7f3d");
            cx.run_until_parked();

            view.update(cx, |view, cx| {
                view.width = 240.;
                cx.notify();
            });
            cx.run_until_parked();
            assert!(cx.debug_bounds("glyph-grid").is_some(), "{picker:?}");
            assert!(cx.debug_bounds("glyph-cell-0").is_none(), "{picker:?}");
        }
    }
}
