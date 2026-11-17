use ely_gpui_component::{
    forms::{
        CodeInput, Combobox, EmojiPicker, FontPicker, IconPicker, JsonInput, NumberInput,
        SignaturePad, Stroke, code_highlights, json_highlights,
    },
    primitives::IconName,
    theme::ActiveTheme,
};
use gpui::{
    App, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::text::field;
use crate::{
    probe::{probe, probe_sized},
    ui::{keep, section, set, specimen, specimens},
};

pub fn signature(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let strokes = keep("signature-strokes", Vec::<Stroke>::new, window, cx);
    let now = strokes.read(cx).clone();
    section(
        "SignaturePad",
        "Sign with the pointer. A stroke lands when the pen lifts; Clear starts over.",
        cx,
    )
    .child(
        specimen(
            match now.len() {
                1 => "1 stroke".to_string(),
                count => format!("{count} strokes"),
            },
            probe_sized(
                "signature",
                div().w_full().child(
                    SignaturePad::new("signature", now)
                        .on_change(move |next, _, cx| set(&strokes, next.to_vec(), cx)),
                ),
                true,
            ),
            cx,
        )
        .w_full()
        .max_w(px(420.0)),
    )
}

pub fn code(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let line = field("input-code", window, cx, |input| {
        input
            .placeholder("cargo run --example gallery")
            .highlighter(code_highlights)
    });
    let json = field("input-json", window, cx, |input| {
        input
            .multi_line(4, 8)
            .placeholder("{ \"name\": \"Ely\" }")
            .highlighter(json_highlights)
    });
    let width = (window.viewport_size().width - px(16.))
        .max(px(0.))
        .min(px(420.));
    section(
        "CodeInput / JsonInput",
        "One line of code, colored the way most languages read. JSON is checked as you type.",
        cx,
    )
    .child(
        specimens()
            .child(specimen(
                "code",
                probe("code", div().w(width).child(CodeInput::new(&line))),
                cx,
            ))
            .child(specimen(
                "json",
                probe("json", div().w(width).child(JsonInput::new(&json))),
                cx,
            )),
    )
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use gpui::{Context, Render, TestAppContext, Window, size};

    use super::*;

    enum Demo {
        Code,
        Font,
        Glyphs,
    }

    struct FormsDemo(Demo);

    impl Render for FormsDemo {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            match self.0 {
                Demo::Code => code(window, cx).into_any_element(),
                Demo::Font => font(window, cx).into_any_element(),
                Demo::Glyphs => glyphs(window, cx).into_any_element(),
            }
        }
    }

    #[gpui::test]
    fn large_stories_fit_the_viewport(cx: &mut TestAppContext) {
        cx.update(ely_gpui_component::init_for_tests);
        let (view, cx) = cx.add_window_view(|_, _| FormsDemo(Demo::Code));
        for (story, keys) in [
            (Demo::Code, &["code", "json"][..]),
            (Demo::Font, &["font"][..]),
        ] {
            view.update(cx, |demo, cx| {
                demo.0 = story;
                cx.notify();
            });
            for (viewport, expected) in [(320., 304.), (1200., 420.)] {
                cx.simulate_resize(size(px(viewport), px(844.)));
                cx.update(|window, cx| {
                    window.refresh();
                    window.draw(cx).clear(cx);
                });
                for &key in keys {
                    let actual = cx.update(|_, cx| {
                        crate::probe::Probes::get(key, cx)
                            .unwrap_or_else(|| panic!("{key} probe"))
                            .size
                            .width
                    });
                    assert_eq!(actual, px(expected), "{key} at viewport {viewport}");
                }
            }
        }
    }

    #[gpui::test]
    fn glyph_pickers_fill_touch_widths(cx: &mut TestAppContext) {
        cx.update(ely_gpui_component::init_for_tests);
        cx.update(|cx| {
            ely_gpui_component::theme::Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, _| FormsDemo(Demo::Glyphs));
        view.update(cx, |demo, cx| {
            demo.0 = Demo::Glyphs;
            cx.notify();
        });
        for (viewport, least) in [(320., 288.), (390., 358.), (768., 448.)] {
            cx.simulate_resize(size(px(viewport), px(844.)));
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
            let mut bounds = |key| {
                cx.update(|_, cx| {
                    crate::probe::Probes::get(key, cx).unwrap_or_else(|| panic!("{key} probe"))
                })
            };
            let (icons, emoji) = (bounds("icons"), bounds("emoji"));
            for (key, actual) in [("icons", icons), ("emoji", emoji)] {
                assert!(
                    actual.size.width >= px(least),
                    "{key} at {viewport}: {actual:?}"
                );
            }
            assert!(emoji.top() >= icons.bottom(), "{icons:?} {emoji:?}");
        }
    }
}

pub fn font(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let search = field("input-font", window, cx, |input| {
        input.placeholder("Search fonts")
    });
    let ui = cx.theme().font_family.clone();
    let family = keep("font-family", move || ui, window, cx);
    let now = family.read(cx).clone();
    let width = (window.viewport_size().width - px(16.))
        .max(px(0.))
        .min(px(420.));
    section(
        "FontPicker",
        "Every family the system knows, and a line set in the chosen one.",
        cx,
    )
    .child(probe(
        "font",
        div().w(width).child(
            FontPicker::new("font-picker", &search)
                .selected(now)
                .on_change(move |next, _, cx| set(&family, next.clone(), cx)),
        ),
    ))
}

pub fn glyphs(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let icon = keep("icon-picked", || IconName::Sparkles, window, cx);
    let emoji = keep("emoji-picked", || SharedString::from("✨"), window, cx);
    let (icon_now, emoji_now) = (*icon.read(cx), emoji.read(cx).clone());
    let touch = cx.theme().touch_mode;
    section(
        "IconPicker / EmojiPicker",
        "Type to narrow. Down moves into the grid, arrows walk it, Enter picks.",
        cx,
    )
    .child(
        specimens()
            .when(touch, |view| view.w_full().min_w_0())
            .child(
                specimen(
                    icon_now.name(),
                    probe_sized(
                        "icons",
                        IconPicker::new("icon-picker")
                            .selected(icon_now)
                            .on_change(move |next, _, cx| set(&icon, next, cx)),
                        touch,
                    ),
                    cx,
                )
                .when(touch, |view| view.w_full().min_w_0()),
            )
            .child(
                specimen(
                    emoji_now.clone(),
                    probe_sized(
                        "emoji",
                        EmojiPicker::new("emoji-picker")
                            .selected(emoji_now)
                            .on_change(move |next, _, cx| {
                                set(&emoji, SharedString::from(next.to_string()), cx)
                            }),
                        touch,
                    ),
                    cx,
                )
                .when(touch, |view| view.w_full().min_w_0()),
            ),
    )
}

pub fn regional(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let language = field("input-language", window, cx, |input| {
        input.placeholder("Language")
    });
    let country = field("input-country", window, cx, |input| {
        input.placeholder("Country")
    });
    let currency = field("input-currency", window, cx, |input| {
        input.placeholder("Currency")
    });
    let spoken = keep("regional-language", || SharedString::from("zh"), window, cx);
    let land = keep("regional-country", || SharedString::from("CN"), window, cx);
    let money = keep(
        "regional-currency",
        || SharedString::from("CNY"),
        window,
        cx,
    );
    let amount = keep("regional-amount", || 1280.0, window, cx);
    let (spoken_now, land_now, money_now, amount_now) = (
        spoken.read(cx).clone(),
        land.read(cx).clone(),
        money.read(cx).clone(),
        *amount.read(cx),
    );
    section(
        "LanguageSelect / CountrySelect / CurrencySelect",
        "Languages under their own names, countries with flags, currencies by ISO 4217. Type a name or a code.",
        cx,
    )
    .child(
        specimens()
            .child(specimen(
                spoken_now.clone(),
                div().w(px(260.0)).child(
                    Combobox::languages("language-select", &language)
                        .selected(spoken_now)
                        .on_change(move |next, _, cx| set(&spoken, next.clone(), cx)),
                ),
                cx,
            ))
            .child(specimen(
                land_now.clone(),
                probe(
                    "country",
                    div().w(px(260.0)).child(
                        Combobox::countries("country-select", &country)
                            .selected(land_now)
                            .on_change(move |next, _, cx| set(&land, next.clone(), cx)),
                    ),
                ),
                cx,
            ))
            .child(specimen(
                money_now.clone(),
                div().w(px(260.0)).child(
                    Combobox::currencies("currency-select", &currency)
                        .selected(money_now.clone())
                        .on_change(move |next, _, cx| set(&money, next.clone(), cx)),
                ),
                cx,
            )),
    )
    .child(specimen(
        format!("an amount in {money_now}"),
        div().w(px(260.0)).child(
            NumberInput::new("regional-amount", amount_now)
                .currency(&money_now)
                .on_change(move |next, _, cx| set(&amount, next, cx)),
        ),
        cx,
    ))
}
