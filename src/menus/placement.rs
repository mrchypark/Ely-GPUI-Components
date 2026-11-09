use gpui::{Bounds, Pixels, Point, Size, point};

pub(super) fn bounded(
    bounds: Bounds<Pixels>,
    from: Point<Pixels>,
    content: impl gpui::IntoElement,
    window: &gpui::Window,
    _: &gpui::App,
) -> Bounded {
    Bounded {
        bounds: area(bounds, window.viewport_size()),
        from,
        child: content.into_any_element(),
    }
}

pub(super) struct Bounded {
    bounds: Bounds<Pixels>,
    from: Point<Pixels>,
    child: gpui::AnyElement,
}

impl gpui::IntoElement for Bounded {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Bounded {
    type RequestLayoutState = gpui::LayoutId;
    type PrepaintState = ();
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, gpui::LayoutId) {
        let child = self.child.request_layout(window, cx);
        let style = gpui::Style {
            position: gpui::Position::Absolute,
            display: gpui::Display::Flex,
            ..Default::default()
        };
        (window.request_layout(style, [child], cx), child)
    }
    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut gpui::LayoutId,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        let size = window.layout_bounds(*child).size;
        let offset = position(self.from, size, self.bounds) - bounds.origin;
        window.with_content_mask(
            Some(gpui::ContentMask {
                bounds: self.bounds,
            }),
            |window| {
                window.with_element_offset(offset, |window| self.child.prepaint(window, cx));
            },
        );
    }
    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut gpui::LayoutId,
        _: &mut (),
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        window.with_content_mask(
            Some(gpui::ContentMask {
                bounds: self.bounds,
            }),
            |window| {
                self.child.paint(window, cx);
            },
        );
    }
}

pub(super) fn root(
    id: &gpui::ElementId,
    menu: &super::model::Menu,
    spot: super::menu::Spot,
    anchor: Bounds<Pixels>,
    content: gpui::Stateful<gpui::Div>,
    window: &gpui::Window,
    cx: &gpui::App,
) -> gpui::AnyElement {
    use super::menu::Spot;
    use crate::{
        motion,
        primitives::raise,
        theme::{ActiveTheme, ControlSize},
    };
    use gpui::{AnimationExt, IntoElement, ParentElement, Styled};
    let from = match spot {
        Spot::Under => anchor.bottom_left(),
        Spot::At(point) => point,
    };
    if let Some(bounds) = menu.usable_bounds {
        return raise(
            (id.clone(), "panel"),
            bounded(bounds, from, content, window, cx),
        )
        .with_priority(1)
        .into_any_element();
    }
    match spot {
        Spot::Under if cx.theme().touch_mode => {
            let theme = cx.theme();
            let rem = window.rem_size();
            let row = theme.touch_minimum(theme.control_height(ControlSize::Md).to_pixels(rem));
            let height = (row * (menu.entries.len() + 1) as f32)
                .min(theme.list_max_height().to_pixels(rem))
                .min(
                    (window.viewport_size().height - theme.window_margin() * 2.0).max(Pixels::ZERO),
                );
            crate::forms::float_height((id.clone(), "panel"), anchor, height, content, window, cx)
        }
        Spot::Under => crate::forms::float(
            (id.clone(), "panel"),
            anchor,
            menu.entries.len(),
            content,
            window,
            cx,
        ),
        Spot::At(point) => raise(
            (id.clone(), "panel"),
            gpui::anchored().position(point).snap_to_window().child(
                content.with_animation(
                    (id.clone(), "in"),
                    gpui::Animation::new(motion::duration(motion::FAST, cx))
                        .with_easing(motion::ease_out_cubic),
                    |root, t| root.opacity(t),
                ),
            ),
        )
        .with_priority(1)
        .into_any_element(),
    }
}

/// Keep the host's exclusions within the current window, including empty usable areas.
pub(super) fn area(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> Bounds<Pixels> {
    let left = bounds.left().max(Pixels::ZERO).min(viewport.width);
    let top = bounds.top().max(Pixels::ZERO).min(viewport.height);
    Bounds::from_corners(
        point(left, top),
        point(
            bounds.right().max(left).min(viewport.width),
            bounds.bottom().max(top).min(viewport.height),
        ),
    )
}

pub(super) fn position(
    from: Point<Pixels>,
    size: Size<Pixels>,
    area: Bounds<Pixels>,
) -> Point<Pixels> {
    point(
        from.x.min(area.right() - size.width).max(area.left()),
        from.y.min(area.bottom() - size.height).max(area.top()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    #[test]
    fn inset_keyboard_area_clamps_root_and_submenu() {
        let usable = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(300.)));
        let panel = size(px(200.), px(280.));
        assert_eq!(
            position(point(px(380.), px(600.)), panel, usable),
            point(px(170.), px(44.))
        );
        assert_eq!(
            position(point(px(-180.), px(0.)), panel, usable),
            usable.origin
        );
    }

    #[test]
    fn short_panels_keep_their_anchor_when_they_fit() {
        let usable = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(300.)));
        for anchor in [point(px(20.), px(150.)), point(px(210.), px(180.))] {
            assert_eq!(position(anchor, size(px(100.), px(60.)), usable), anchor);
        }
    }

    #[test]
    fn usable_area_is_clipped_and_can_be_empty() {
        let viewport = size(px(390.), px(600.));
        assert_eq!(
            area(
                Bounds::new(point(px(-10.), px(24.)), size(px(500.), px(800.))),
                viewport
            ),
            Bounds::new(point(px(0.), px(24.)), size(px(390.), px(576.)))
        );
        assert_eq!(
            area(
                Bounds::new(point(px(400.), px(700.)), size(px(10.), px(10.))),
                viewport
            )
            .size,
            size(px(0.), px(0.))
        );
    }

    #[test]
    fn filtered_menu_retains_host_bounds() {
        use crate::menus::{Menu, MenuItem};
        let bounds = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(300.)));
        let menu = Menu::new().usable_bounds(bounds).item(MenuItem::new("Map"));
        assert_eq!(menu.filtered("Map").usable_bounds, Some(bounds));
        assert_eq!(menu.filtered("missing").usable_bounds, Some(bounds));
        assert_eq!(Menu::new().filtered("").usable_bounds, None);
    }
}
