use ely_gpui_component::layout::{
    keyboard_reveal_region, reveal_in_bounds, reveal_offset, usable_bounds,
};
use gpui::{Bounds, Pixels, point, px};

pub(super) fn keyboard_occlusion(docked: Pixels, platform_ime: Pixels, floating: bool) -> Pixels {
    if floating {
        docked
    } else {
        docked.max(platform_ime.max(Pixels::ZERO))
    }
}

pub(super) fn needed_top_reach(
    field: Bounds<Pixels>,
    visible: Bounds<Pixels>,
    keyboard: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
) -> Pixels {
    if keyboard.is_none_or(|keyboard| {
        visible.top() < keyboard.bottom()
            || field.right() <= keyboard.left()
            || field.left() >= keyboard.right()
            || field.top() >= keyboard.bottom()
    }) {
        return Pixels::ZERO;
    }
    (scroll_y + reveal_offset(field, visible).y).max(Pixels::ZERO)
}

pub(super) fn keyboard_target(
    element: Option<Bounds<Pixels>>,
    caret: Option<Bounds<Pixels>>,
    frame: Bounds<Pixels>,
    keyboard: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
    max_scroll_y: Pixels,
) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
    [element, caret].into_iter().flatten().find_map(|bounds| {
        let field = Bounds::new(
            point(bounds.left(), bounds.top() - px(16.)),
            gpui::size(bounds.size.width.max(px(1.)), bounds.size.height + px(32.)),
        );
        if field.size.height > frame.size.height {
            return None;
        }
        let preferred = keyboard_reveal_region(field, frame, keyboard, px(0.))?;
        let reachable = |visible| {
            let next = scroll_y + reveal_offset(field, visible).y;
            next >= -max_scroll_y && next <= px(0.)
        };
        if reachable(preferred) {
            return Some((field, preferred));
        }
        let alternate = keyboard.and_then(|keyboard| {
            if preferred.bottom() <= keyboard.top() {
                let top = keyboard.bottom().clamp(frame.top(), frame.bottom());
                Some(Bounds::new(
                    point(frame.left(), top),
                    gpui::size(frame.size.width, frame.bottom() - top),
                ))
            } else if preferred.top() >= keyboard.bottom() {
                let bottom = keyboard.top().clamp(frame.top(), frame.bottom());
                Some(Bounds::new(
                    frame.origin,
                    gpui::size(frame.size.width, bottom - frame.top()),
                ))
            } else {
                None
            }
        });
        let visible = alternate
            .filter(|region| region.size.height >= field.size.height && reachable(*region))
            .unwrap_or(preferred);
        Some((field, visible))
    })
}

impl super::MobileGallery {
    pub(super) fn retry_keyboard_reveal(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        if self.reveal_retries == 0 {
            return false;
        }
        self.reveal_retries -= 1;
        cx.notify();
        true
    }

    pub(super) fn reclaim_top_reach(&mut self, keyboard: Pixels) -> bool {
        let offset = self.scroll.offset();
        let removable = self.top_reach.min((-offset.y).max(px(0.)));
        if removable <= px(0.) {
            if keyboard <= px(0.)
                && self.floating_keyboard.is_none()
                && self.top_reach > px(0.)
                && self.scroll.max_offset().y <= px(0.)
            {
                // Drop the orphaned spacer when no scroll range remains to compensate.
                self.top_reach = px(0.);
                return true;
            }
            return false;
        }
        self.top_reach -= removable;
        self.scroll
            .set_offset(point(offset.x, offset.y + removable));
        true
    }

    pub(super) fn schedule_keyboard_reveal(
        &mut self,
        window: &gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let viewport = window.viewport_size();
        let viewport = (viewport.width, viewport.height);
        if self.reveal_viewport != Some(viewport) {
            self.reveal_viewport = Some(viewport);
            self.reveal_retries = 1;
        }
        // Android can expose the IME edge before the host receives a keyboard rectangle.
        let keyboard = keyboard_occlusion(
            self.keyboard,
            window.insets().ime.bottom,
            self.floating_keyboard.is_some(),
        );
        if !self.reveal_pending
            && (keyboard > px(0.) || self.floating_keyboard.is_some() || self.top_reach > px(0.))
        {
            self.reveal_pending = true;
            let weak = cx.weak_entity();
            let focus = window.focused(cx);
            let expected = (
                self.insets,
                keyboard,
                self.floating_keyboard,
                self.top_reach,
                self.scroll.offset(),
                viewport,
                window.fully_visible_bounds(),
            );
            window.on_next_frame(move |window, cx| {
                let _ = weak.update(cx, |view, cx| {
                    view.reveal_pending = false;
                    let keyboard = keyboard_occlusion(
                        view.keyboard,
                        window.insets().ime.bottom,
                        view.floating_keyboard.is_some(),
                    );
                    if (
                        view.insets,
                        keyboard,
                        view.floating_keyboard,
                        view.top_reach,
                        view.scroll.offset(),
                        (window.viewport_size().width, window.viewport_size().height),
                        window.fully_visible_bounds(),
                    ) != expected
                        || window.focused(cx) != focus
                    {
                        if keyboard > px(0.)
                            || view.floating_keyboard.is_some()
                            || view.top_reach > px(0.)
                        {
                            view.retry_keyboard_reveal(cx);
                        }
                        return;
                    }
                    // Reclaim compensated space independently of overlay ownership.
                    if view.floating_keyboard.is_none() && view.reclaim_top_reach(keyboard) {
                        cx.notify();
                        return;
                    }
                    if view.floating_keyboard.is_none() && keyboard <= px(0.) {
                        return;
                    }
                    let Some((element, caret)) = window.focused_text_input_geometry() else {
                        view.retry_keyboard_reveal(cx);
                        return;
                    };
                    if ely_gpui_component::primitives::focused_in_takeover(window, cx)
                        || !view.focus.contains_focused(window, cx)
                    {
                        return;
                    }
                    if let (Some(floating_keyboard), Some(bounds)) =
                        (view.floating_keyboard, element.or(caret))
                    {
                        let field = Bounds::new(
                            point(bounds.left(), bounds.top() - px(16.)),
                            gpui::size(bounds.size.width.max(px(1.)), bounds.size.height + px(32.)),
                        );
                        let overlap = field.intersect(&floating_keyboard);
                        if (overlap.size.width <= px(0.) || overlap.size.height <= px(0.))
                            && view.reclaim_top_reach(keyboard)
                        {
                            cx.notify();
                            return;
                        }
                    }
                    let frame = usable_bounds(
                        Bounds::new(point(px(0.), px(0.)), window.viewport_size()),
                        view.insets,
                        keyboard,
                    )
                    .intersect(&window.fully_visible_bounds())
                    .intersect(&view.scroll.bounds());
                    let Some((field, visible)) = keyboard_target(
                        element,
                        caret,
                        frame,
                        view.floating_keyboard,
                        view.scroll.offset().y,
                        view.scroll.max_offset().y,
                    ) else {
                        return;
                    };
                    let extra = needed_top_reach(
                        field,
                        visible,
                        view.floating_keyboard,
                        view.scroll.offset().y,
                    );
                    if extra > px(0.) {
                        let offset = view.scroll.offset();
                        view.top_reach += extra;
                        view.scroll.set_offset(point(offset.x, offset.y - extra));
                        cx.notify();
                        return;
                    }
                    if reveal_in_bounds(&view.scroll, field, visible) {
                        cx.notify();
                    }
                });
            });
        }
    }
}
