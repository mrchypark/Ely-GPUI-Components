//! UIKit entry points for the shared mobile gallery.

macro_rules! asset {
    ($name:literal) => {
        concat!("gallery-assets/", $name)
    };
}

// These modules are the shared gallery crate root, not copied gallery code.
#[path = "../../mobile_gallery/assets.rs"]
pub mod assets;
#[path = "../../gallery/menu_model.rs"]
mod menu_model;
#[path = "../../gallery/pages/mod.rs"]
pub mod pages;
#[path = "../../gallery/probe.rs"]
pub mod probe;
#[path = "../../gallery/step.rs"]
pub mod step;
#[path = "../../gallery/ui.rs"]
pub mod ui;
#[path = "../../mobile_gallery/view.rs"]
mod view;

pub use view::MobileGallery;

pub fn app_menus() -> Vec<gpui::Menu> {
    vec![menu_model::view_menu()]
}

pub fn init(cx: &mut gpui::App) {
    use ely_gpui_component::theme::{Mode, Theme};
    cx.on_action(|_: &menu_model::LightMode, cx| Theme::set_mode_now(Mode::Light, cx));
    cx.on_action(|_: &menu_model::DarkMode, cx| Theme::set_mode_now(Mode::Dark, cx));
    Theme::update(cx, |theme| {
        theme.touch_mode = true;
        theme.font_family = ".SystemUIFont".into();
    });
    pages::bind_keys(cx);
    pages::install_i18n(cx);
}

use ely_gpui_component::layout::ViewportInsets;
use gpui::{
    App, Bounds, Context, Entity, IntoElement, Render, Window, WindowOptions, div, prelude::*, px,
};
use gpui_mobile::mobile_events::{MobileEvent, WindowInsets, set_mobile_event_handler};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

struct GalleryHost {
    gallery: Entity<MobileGallery>,
    insets: WindowInsets,
    applied: Option<(ViewportInsets, gpui::Pixels, Option<Bounds<gpui::Pixels>>)>,
}

impl GalleryHost {
    fn apply_insets(&mut self, window: &Window, cx: &mut Context<Self>) {
        let safe = ViewportInsets {
            top: self.insets.safe_area.top,
            right: self.insets.safe_area.right,
            bottom: self.insets.safe_area.bottom,
            left: self.insets.safe_area.left,
        };
        let keyboard = self
            .insets
            .keyboard
            .filter(|_| self.insets.keyboard_visible != Some(false));
        let docked = keyboard
            .filter(|bounds| (bounds.bottom() - window.viewport_size().height).abs() <= px(0.5));
        let update = (
            safe,
            docked.map_or(px(0.), |bounds| bounds.size.height),
            keyboard.filter(|bounds| Some(*bounds) != docked),
        );
        if self.applied == Some(update) {
            return;
        }
        self.gallery.update(cx, |gallery, cx| {
            gallery.set_insets(update.0, update.1, cx);
            gallery.set_floating_keyboard(update.2, cx);
        });
        self.applied = Some(update);
    }
}

impl Render for GalleryHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_insets(window, cx);
        div().size_full().child(self.gallery.clone())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_ios_register_app() {
    gpui_mobile::ios::ffi::set_app_callback(Box::new(open_window));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_ios_run_chart_gallery() {
    gpui_mobile::ios::ffi::run_app_with_assets(assets::GalleryAssets);
}

fn open_window(cx: &mut App) {
    let root = gpui_mobile::packages::path_provider::cache_directory()
        .expect("find app cache directory")
        .join("ely-mobile-gallery-assets");
    assets::prepare_files(&root).expect("materialize bundled gallery assets");
    ely_gpui_component::init(cx).expect("initialize Ely");
    init(cx);
    cx.open_window(WindowOptions::default(), |window, cx| {
        let gallery = cx.new(|cx| {
            let mut gallery = MobileGallery::new(window, cx);
            gallery.select("charts", cx);
            gallery
        });
        let host = cx.new(|_| GalleryHost {
            gallery,
            insets: WindowInsets::default(),
            applied: None,
        });
        let weak = host.downgrade();
        let app = cx.to_async();
        let executor = cx.foreground_executor().clone();
        let pending = Rc::new(RefCell::new(VecDeque::new()));
        let scheduled = Rc::new(Cell::new(false));
        set_mobile_event_handler(Some(Box::new(move |_, event| {
            let MobileEvent::Insets(insets) = event else {
                return;
            };
            pending.borrow_mut().push_back(insets.clone());
            if scheduled.replace(true) {
                return;
            }
            let (pending, scheduled, weak, mut app) = (
                pending.clone(),
                scheduled.clone(),
                weak.clone(),
                app.clone(),
            );
            executor
                .spawn(async move {
                    let latest = pending.borrow_mut().drain(..).last();
                    scheduled.set(false);
                    if let Some(insets) = latest {
                        let _ = weak.update(&mut app, |host, cx| {
                            host.insets = insets;
                            cx.notify();
                        });
                    }
                })
                .detach();
        })));
        host
    })
    .expect("open gallery window");
}
