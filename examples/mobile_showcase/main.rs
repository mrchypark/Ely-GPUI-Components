//! Desktop harness for the backend-independent view in `view.rs`.
mod view;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn main() {
    gpui_platform::application()
        .with_assets(ely_gpui_component::Assets)
        .run(|cx: &mut gpui::App| {
            use gpui::AppContext;
            ely_gpui_component::init(cx).expect("Ely failed to start");
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                cx.new(|cx| view::Showcase::new(window, cx))
            })
            .expect("showcase window failed to open");
            cx.activate(true);
        });
}
