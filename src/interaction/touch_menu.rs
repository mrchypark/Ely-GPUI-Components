use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, StyleRefinement, Styled, Window, div, point, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    layout::{ViewportInsets, usable_bounds},
    menus::{Menu, Open, Spot, hang, measure_host},
    primitives::Pressable,
    theme::ActiveTheme,
};

/// Its children, and a menu that opens where a long press lands. The panel is the
/// menus engine's own, so it occludes: a press on the menu never reaches what it covers.
#[derive(IntoElement)]
pub struct TouchMenu {
    id: ElementId,
    menu: Menu,
    children: SmallVec<[AnyElement; 2]>,
    base: Div,
    insets: Option<ViewportInsets>,
    keyboard_bottom: Pixels,
    long_press: bool,
}

impl TouchMenu {
    pub fn new(id: impl Into<ElementId>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            menu,
            children: SmallVec::new(),
            base: div(),
            insets: None,
            keyboard_bottom: Pixels::ZERO,
            long_press: true,
        }
    }

    /// The menu the long press opens.
    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = menu;
        self
    }

    /// The safe area and keyboard occlusion the host reports, in logical window coordinates.
    pub fn viewport_insets(mut self, insets: ViewportInsets, keyboard_bottom: Pixels) -> Self {
        self.insets = Some(insets);
        self.keyboard_bottom = keyboard_bottom;
        self
    }

    /// The window area the panel may use, in logical window coordinates.
    pub fn usable_bounds(mut self, bounds: gpui::Bounds<Pixels>) -> Self {
        self.menu = self.menu.usable_bounds(bounds);
        self
    }

    /// Whether a long press opens the menu. On by default, and off where the owner wants a press alone.
    pub fn long_press(mut self, long_press: bool) -> Self {
        self.long_press = long_press;
        self
    }
}

impl ParentElement for TouchMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TouchMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for TouchMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "menu"), cx, |_, _| Open::default());
        let menu = match self.insets {
            Some(insets) => self.menu.usable_bounds(usable_bounds(
                gpui::Bounds::new(point(Pixels::ZERO, Pixels::ZERO), window.viewport_size()),
                insets,
                self.keyboard_bottom,
            )),
            None => self.menu,
        };
        let (open, shut, menu) = (state.clone(), state.clone(), menu.clone());
        let shown = menu.clone();
        let touch = self.long_press && cx.theme().touch_mode;
        let panel = hang(&self.id, &menu, &state, None, window, cx);
        let theme = cx.theme();
        self.base
            .id(self.id.clone())
            .relative()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if shut.read(cx).is_open() {
                    window.prevent_default();
                    Open::close(&shut, window, cx);
                }
            })
            .when(touch, |host| {
                host.child(
                    Pressable::new((self.id.clone(), "touch"), |_, _, _| {
                        div().absolute().size_full().into_any_element()
                    })
                    .absolute()
                    .size_full()
                    .on_long_press(move |window, cx| {
                        let at = window.mouse_position();
                        log::info!("touch menu: long press at {at:?}");
                        Open::show(&open, &shown, Spot::At(at), false, cx);
                    }),
                )
            })
            .children(self.children)
            .child(measure_host(state.clone(), false))
            .children(panel)
            .map(|host| theme.touch_target(host))
    }
}
