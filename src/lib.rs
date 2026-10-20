pub mod account;
pub mod agent;
mod assets;

pub mod buttons;
pub mod calendar;
pub mod canvas;
pub mod charts;
pub mod chat;
pub mod collab;
pub mod dashboard;
pub mod data_display;
pub mod debug;
pub mod devtools;
pub mod documents;
pub mod editor;
pub mod feedback;
pub mod files;
pub mod finance;
pub mod forms;
pub mod generative;
pub mod git;
pub mod i18n;
pub mod interaction;
pub mod layout;
pub mod lists;
pub mod mail;
pub mod maps;
pub mod media;
pub mod menus;
pub mod messaging;
pub mod misc;
pub mod motion;
pub mod navigation;
pub mod onboarding;
pub mod overlays;
pub mod primitives;
pub mod project;
pub mod rendering;
pub mod settings;
pub mod shell;
pub mod tables;
pub mod terminal;
pub mod theme;
pub mod tooling;
pub mod typography;

#[cfg(all(test, feature = "test-support"))]
mod touch_targets;

pub use assets::Assets;

use anyhow::Context as _;
use gpui::{App, KeyBinding};

use crate::primitives::{FocusNext, FocusPrev, IconName};

/// Loads fonts and the theme, binds Tab and text keys. Call once, first.
pub fn init(cx: &mut App) -> anyhow::Result<()> {
    match cx.asset_source().load(IconName::Check.path()) {
        Ok(Some(_)) => {}
        Ok(None) => {
            anyhow::bail!("ely: pass `ely_gpui_component::Assets` to `Application::with_assets`")
        }
        Err(error) => return Err(error.context("ely: asset source failed")),
    }
    assets::load_fonts(cx).context("ely: embedded fonts failed to register")?;
    setup(cx);
    Ok(())
}

/// `init` without assets or fonts, for app tests that render.
#[cfg(any(test, feature = "test-support"))]
pub fn init_for_tests(cx: &mut App) {
    setup(cx);
}

/// The theme and key bindings.
fn setup(cx: &mut App) {
    theme::Theme::init(cx);
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrev, None),
    ]);
    forms::bind_keys(cx);
    editor::bind_keys(cx);
    #[cfg(not(target_family = "wasm"))]
    terminal::bind_keys(cx);
    documents::bind_keys(cx);
}
