use gpui::{
    AnyElement, Bounds, Context, FocusHandle, IntoElement, KeyUpEvent, Keystroke, ParentElement,
    Pixels, Render, Styled, TestAppContext, VisualTestContext, Window, div, hsla, point,
    prelude::*, px, size,
};

use super::{Border, BorderEditor, Dash, Shadow, ShadowEditor};
use crate::{
    devtools::ColorContrastChecker,
    primitives::FocusScope,
    settings::{AccentColorPicker, ThemeDraft, ThemeEditor},
    theme::{ActiveTheme, Theme},
};

#[derive(Clone, Copy)]
enum Well {
    Shadow,
    Border,
    Theme,
    Accent,
    Text,
    Background,
}

impl Well {
    fn tabs(self) -> usize {
        match self {
            Self::Shadow | Self::Accent | Self::Background => 2,
            Self::Theme => 8,
            Self::Border | Self::Text => 1,
        }
    }
}

struct Editor {
    root: FocusHandle,
    well: Well,
    bounds: Option<Bounds<Pixels>>,
    changes: Vec<String>,
    color: Option<gpui::Hsla>,
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let record = move |color: gpui::Hsla, cx: &mut gpui::App| {
            owner.update(cx, |view, cx| {
                view.changes.push(crate::forms::hex(color.to_rgb()));
                view.color = Some(color);
                cx.notify();
            });
        };
        let red = hsla(0., 0.8, 0.5, 1.);
        let blue = hsla(0.6, 0.8, 0.5, 1.);
        let chosen = self.color.unwrap_or(red);
        let bounds = self.bounds;
        let part: AnyElement = match self.well {
            Well::Shadow => ShadowEditor::new(
                "shadow",
                [Shadow {
                    x: 3.,
                    y: 4.,
                    blur: 5.,
                    spread: 6.,
                    color: chosen,
                }],
                move |shadows, _, cx| {
                    assert_eq!(shadows.len(), 1);
                    let shadow = shadows[0];
                    assert_eq!(
                        (shadow.x, shadow.y, shadow.blur, shadow.spread),
                        (3., 4., 5., 6.)
                    );
                    record(shadow.color, cx);
                },
            )
            .when_some(bounds, |editor, bounds| editor.usable_bounds(bounds))
            .into_any_element(),
            Well::Border => BorderEditor::new(
                "border",
                Border {
                    width: 3.,
                    dash: Dash::Solid,
                    color: chosen,
                    radii: [4.; 4],
                },
                move |border, _, cx| {
                    assert_eq!(border.width, 3.);
                    assert_eq!(border.dash, Dash::Solid);
                    assert_eq!(border.radii, [4.; 4]);
                    record(border.color, cx);
                },
            )
            .when_some(bounds, |editor, bounds| editor.usable_bounds(bounds))
            .into_any_element(),
            Well::Theme => {
                let mut draft = ThemeDraft::of(cx.theme());
                if let Some(color) = self.color {
                    draft.colors.bg = color;
                }
                let original = draft.clone();
                ThemeEditor::new("theme", draft, move |draft, _, cx| {
                    let mut expected = original.clone();
                    expected.colors.bg = draft.colors.bg;
                    assert_eq!(*draft, expected);
                    record(draft.colors.bg, cx);
                })
                .when_some(bounds, |editor, bounds| editor.usable_bounds(bounds))
                .into_any_element()
            }
            Well::Accent => {
                AccentColorPicker::new("accent", chosen, [("Blue", blue)], move |color, _, cx| {
                    record(color, cx)
                })
                .when_some(bounds, |editor, bounds| editor.usable_bounds(bounds))
                .into_any_element()
            }
            Well::Text | Well::Background => {
                let text = matches!(self.well, Well::Text);
                ColorContrastChecker::new(
                    "contrast",
                    if text { chosen } else { red },
                    if text {
                        blue
                    } else {
                        self.color.unwrap_or(blue)
                    },
                    move |foreground, background, _, cx| {
                        if text {
                            assert_eq!(background, blue);
                            record(foreground, cx);
                        } else {
                            assert_eq!(foreground, red);
                            record(background, cx);
                        }
                    },
                )
                .when_some(bounds, |editor, bounds| editor.usable_bounds(bounds))
                .into_any_element()
            }
        };
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().w(px(640.)).child(part))
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    settle(cx);
}

fn contained(cx: &mut VisualTestContext, bounds: Bounds<Pixels>) {
    let panel = cx.debug_bounds("popover-panel").expect("color well opened");
    assert!(
        panel.left() >= bounds.left() && panel.top() >= bounds.top(),
        "{panel:?} in {bounds:?}"
    );
    assert!(
        panel.right() <= bounds.right() && panel.bottom() <= bounds.bottom(),
        "{panel:?} in {bounds:?}"
    );
}

fn exercise(well: Well, cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let initial = Bounds::new(point(px(20.), px(30.)), size(px(280.), px(300.)));
    let (view, cx) = cx.add_window_view(|window, cx| {
        let root = cx.focus_handle();
        window.focus(&root, cx);
        Editor {
            root,
            well,
            bounds: Some(initial),
            changes: Vec::new(),
            color: None,
        }
    });
    settle(cx);
    for _ in 0..well.tabs() {
        press("tab", cx);
    }
    press("enter", cx);
    contained(cx, initial);
    let smaller = Bounds::new(point(px(40.), px(60.)), size(px(220.), px(180.)));
    view.update(cx, |view, cx| {
        view.bounds = Some(smaller);
        cx.notify();
    });
    settle(cx);
    contained(cx, smaller);
    for _ in 0..if cfg!(target_os = "macos") { 3 } else { 2 } {
        press("tab", cx);
    }
    press("secondary-a", cx);
    cx.simulate_input("#224466");
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.changes.clone()),
        ["#224466"]
    );
    contained(cx, smaller);

    view.update(cx, |view, cx| {
        view.bounds = Some(Bounds::new(point(px(40.), px(60.)), size(px(0.), px(0.))));
        cx.notify();
    });
    settle(cx);
    assert!(
        cx.debug_bounds("popover-panel").is_none(),
        "empty bounds close the picker"
    );
    view.update(cx, |view, cx| {
        view.bounds = Some(initial);
        cx.notify();
    });
    settle(cx);
    assert!(
        cx.debug_bounds("popover-panel").is_none(),
        "restoring bounds does not reopen"
    );
    let root = view.read_with(cx, |view, _| view.root.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    for _ in 0..well.tabs() {
        press("tab", cx);
    }
    press("enter", cx);
    contained(cx, initial);
    press("escape", cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    assert_eq!(
        view.read_with(cx, |view, _| view.changes.clone()),
        ["#224466"]
    );
}

#[gpui::test]
fn shadow_well_forwards_live_bounds_and_preserves_shadow(cx: &mut TestAppContext) {
    exercise(Well::Shadow, cx);
}

#[gpui::test]
fn border_well_forwards_live_bounds_and_preserves_border(cx: &mut TestAppContext) {
    exercise(Well::Border, cx);
}

#[gpui::test]
fn theme_well_forwards_live_bounds_and_preserves_other_tokens(cx: &mut TestAppContext) {
    exercise(Well::Theme, cx);
}

#[gpui::test]
fn accent_well_forwards_live_bounds_and_reports_color_once(cx: &mut TestAppContext) {
    exercise(Well::Accent, cx);
}

#[gpui::test]
fn contrast_text_well_forwards_live_bounds_and_preserves_background(cx: &mut TestAppContext) {
    exercise(Well::Text, cx);
}

#[gpui::test]
fn contrast_background_well_forwards_live_bounds_and_preserves_text(cx: &mut TestAppContext) {
    exercise(Well::Background, cx);
}
