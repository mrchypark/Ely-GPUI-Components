use gpui::prelude::FluentBuilder as _;
use std::collections::HashMap;

use ely_gpui_component::primitives::Measure;
use gpui::{
    AnyWindowHandle, App, Bounds, Div, Global, IntoElement, ParentElement, Pixels, Styled, div,
};

/// Window bounds of named demo elements, for scripted capture.
#[derive(Default)]
pub struct Probes(HashMap<&'static str, Bounds<Pixels>>);

impl Global for Probes {}

impl Probes {
    #[cfg(not(target_family = "wasm"))]
    pub fn get(key: &str, cx: &App) -> Option<Bounds<Pixels>> {
        cx.try_global::<Probes>()
            .and_then(|probes| probes.0.get(key).copied())
    }
}

/// Wraps `element`, sized to it, so scripts can find it by `key`.
pub fn probe(key: &'static str, element: impl IntoElement) -> Div {
    probe_sized(key, element, false)
}

/// Fill an allocated caller width without letting the measurement flex item
/// impose its child's intrinsic width. Existing probes retain content sizing.
pub fn probe_sized(key: &'static str, element: impl IntoElement, fill: bool) -> Div {
    div().flex().child(
        Measure::new(key, move |bounds, _, cx| {
            cx.default_global::<Probes>().0.insert(key, bounds);
        })
        .when(fill, |measure| measure.flex_1().min_w_0().w_full())
        .child(element),
    )
}

/// Windows the demos opened, by key, for scripted capture.
#[derive(Default)]
pub struct Opened(HashMap<&'static str, AnyWindowHandle>);

impl Global for Opened {}

impl Opened {
    pub fn insert(key: &'static str, handle: AnyWindowHandle, cx: &mut App) {
        cx.default_global::<Opened>().0.insert(key, handle);
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn take(key: &str, cx: &mut App) -> Option<AnyWindowHandle> {
        cx.default_global::<Opened>().0.remove(key)
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn get(key: &str, cx: &App) -> Option<AnyWindowHandle> {
        cx.try_global::<Opened>()
            .and_then(|opened| opened.0.get(key).copied())
    }
}
