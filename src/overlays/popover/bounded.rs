use gpui::{
    AnyElement, App, Bounds, Display, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, Position, Rems, Size, Style, Window, point, px, rems,
};

use crate::primitives::raise;

pub(in crate::overlays) const POPOVER_PADDING: Rems = rems(0.75);
pub(in crate::overlays) const HOVER_PADDING: Rems = rems(1.);

pub(crate) fn has_room(area: Option<Bounds<Pixels>>, padding: Rems, window: &Window) -> bool {
    // Both surfaces use a one-pixel border; reserve a positive content viewport.
    let frame = padding.to_pixels(window.rem_size()) * 2. + px(2.);
    area.is_none_or(|area| area.size.width > frame && area.size.height > frame)
}

pub(crate) fn available(
    supplied: Option<Bounds<Pixels>>,
    touch: bool,
    window: &Window,
) -> Option<Bounds<Pixels>> {
    let viewport = window.viewport_size();
    supplied
        .or_else(|| touch.then(|| Bounds::new(Point::default(), viewport)))
        .map(|bounds| {
            let left = bounds.left().max(Pixels::ZERO).min(viewport.width);
            let top = bounds.top().max(Pixels::ZERO).min(viewport.height);
            Bounds::from_corners(
                point(left, top),
                point(
                    bounds.right().max(left).min(viewport.width),
                    bounds.bottom().max(top).min(viewport.height),
                ),
            )
        })
}

pub(crate) fn place(
    id: ElementId,
    anchor: Bounds<Pixels>,
    area: Bounds<Pixels>,
    child: impl IntoElement,
) -> AnyElement {
    raise(
        id,
        Bounded {
            anchor,
            area,
            child: child.into_any_element(),
        },
    )
    .with_priority(1)
    .into_any_element()
}

fn origin(anchor: Bounds<Pixels>, size: Size<Pixels>, area: Bounds<Pixels>) -> Point<Pixels> {
    let below = area.bottom() - anchor.bottom();
    let above = anchor.top() - area.top();
    let y = if size.height > below && above > below {
        anchor.top() - size.height
    } else {
        anchor.bottom()
    };
    point(
        anchor
            .left()
            .min(area.right() - size.width)
            .max(area.left()),
        y.min(area.bottom() - size.height).max(area.top()),
    )
}

struct Bounded {
    anchor: Bounds<Pixels>,
    area: Bounds<Pixels>,
    child: AnyElement,
}

impl IntoElement for Bounded {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Bounded {
    type RequestLayoutState = LayoutId;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let child = self.child.request_layout(window, cx);
        let style = Style {
            position: Position::Absolute,
            display: Display::Flex,
            ..Default::default()
        };
        (window.request_layout(style, [child], cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let offset =
            origin(self.anchor, window.layout_bounds(*child).size, self.area) - bounds.origin;
        window.with_element_offset(offset, |window| self.child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    #[test]
    fn placement_uses_current_size_and_inset_edges() {
        let area = Bounds::new(point(px(20.), px(30.)), size(px(280.), px(300.)));
        let anchor = Bounds::new(point(px(270.), px(280.)), size(px(40.), px(40.)));
        assert_eq!(
            origin(anchor, size(px(200.), px(100.)), area),
            point(px(100.), px(180.))
        );
        assert_eq!(origin(anchor, area.size, area), area.origin);
        let outside = Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.)));
        assert_eq!(origin(outside, area.size, area), area.origin);
    }
}
