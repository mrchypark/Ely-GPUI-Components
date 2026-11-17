use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    data_display::CountBadge,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

type OnSelect = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One destination in a mobile bottom navigation bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomNavItem {
    pub id: SharedString,
    pub icon: IconName,
    pub label: SharedString,
    pub badge: Option<usize>,
}

impl BottomNavItem {
    pub fn new(
        id: impl Into<SharedString>,
        icon: IconName,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            badge: None,
        }
    }

    /// Optional unread or pending count shown over the icon.
    pub fn badge(mut self, count: usize) -> Self {
        self.badge = Some(count);
        self
    }
}

/// Mobile-first bottom tab navigation bar with optional badges and safe-area bottom inset.
#[derive(IntoElement)]
pub struct BottomNavBar {
    id: ElementId,
    items: Vec<BottomNavItem>,
    selected: SharedString,
    bottom_inset: Pixels,
    on_select: Option<OnSelect>,
}

impl BottomNavBar {
    pub fn new(
        id: impl Into<ElementId>,
        items: impl IntoIterator<Item = BottomNavItem>,
        selected: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().collect(),
            selected: selected.into(),
            bottom_inset: Pixels::ZERO,
            on_select: None,
        }
    }

    /// Extra bottom padding for the device safe area (e.g. home indicator).
    pub fn bottom_inset(mut self, inset: Pixels) -> Self {
        self.bottom_inset = inset;
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BottomNavBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let bar_id = self.id.clone();

        div()
            .id(self.id)
            .debug_selector(|| "bottom-nav-bar".into())
            .w_full()
            .flex()
            .items_stretch()
            .bg(colors.surface)
            .border_t_1()
            .border_color(colors.border)
            .pt_1()
            .pb(self.bottom_inset)
            .children(self.items.into_iter().map(|item| {
                let active = item.id == self.selected;
                let fg = if active {
                    colors.accent
                } else {
                    colors.fg_muted
                };
                let icon = Icon::new(item.icon).size(IconSize::Md).color(fg);
                let icon_box = match item.badge {
                    Some(count) => {
                        CountBadge::new((bar_id.clone(), format!("badge-{}", item.id)), count)
                            .over(icon)
                            .into_any_element()
                    }
                    None => icon.into_any_element(),
                };
                let item_id = item.id.clone();
                let selector_id = item.id.clone();
                let on_select = self.on_select.clone();

                div()
                    .id((bar_id.clone(), item.id))
                    .debug_selector(move || format!("bottom-nav-{}", selector_id))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .py_1()
                    .rounded(theme.radius(Radius::Md))
                    .map(|el| theme.touch_target(el))
                    .cursor_pointer()
                    .tab_index(0)
                    .focus_ring(cx)
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("bottom nav: {item_id}");
                        if let Some(select) = &on_select {
                            select(&item_id, window, cx);
                        }
                    })
                    .child(icon_box)
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(fg)
                            .when(active, |label| label.font_weight(FontWeight::MEDIUM))
                            .truncate()
                            .child(item.label),
                    )
            }))
    }
}
