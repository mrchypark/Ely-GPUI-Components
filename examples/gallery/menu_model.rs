use gpui::{Menu, MenuItem, actions};

actions!(gallery, [LightMode, DarkMode]);

/// The same actions can be drawn by Ely or installed in a desktop system menu.
pub fn view_menu() -> Menu {
    Menu {
        name: "View".into(),
        items: vec![
            MenuItem::action("Light", LightMode),
            MenuItem::action("Dark", DarkMode),
        ],
        disabled: false,
    }
}
