use crate::{
    data_display::Avatar,
    forms::{AvatarUpload, Cascade, Cascader, Choice, ChoiceChips, Rating},
    theme::Theme,
};
use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    Window, div, point, px,
};

/// The theme as a phone has it: no hover, no motion.
fn touch_theme(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
}

struct Picture;
impl Render for Picture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(AvatarUpload::new(
            "picture",
            Avatar::new("face", "Ada Lovelace"),
        ))
    }
}

#[gpui::test]
fn a_touch_avatar_shows_its_camera_without_a_hover(cx: &mut TestAppContext) {
    touch_theme(cx);
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Picture
    });
    cx.run_until_parked();
    let camera = cx
        .debug_bounds("avatar-camera-button")
        .expect("the camera shows");
    assert!(
        camera.size.width >= px(48.) && camera.size.height >= px(48.),
        "the camera is {camera:?}"
    );
    assert!(
        cx.debug_bounds("avatar-camera-overlay-touch").is_some(),
        "the overlay shows in touch mode"
    );
}

struct Chips;
impl Render for Chips {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(ChoiceChips::new(
            "chips",
            [Choice::new("a", "Alpha"), Choice::new("b", "Beta")],
        ))
    }
}

#[gpui::test]
fn a_choice_chip_is_big_enough_to_tap(cx: &mut TestAppContext) {
    touch_theme(cx);
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Chips
    });
    cx.run_until_parked();
    let chip = cx.debug_bounds("choice-chip-0").expect("a chip draws");
    assert!(
        chip.size.width >= px(48.) && chip.size.height >= px(48.),
        "the chip is {chip:?}"
    );
}

struct Stars;
impl Render for Stars {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(Rating::new("rating", 2))
    }
}

#[gpui::test]
fn a_tap_on_a_star_shows_the_preview_in_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Stars
    });
    cx.run_until_parked();
    for (n, selector) in [
        (1, "rating-star-1"),
        (2, "rating-star-2"),
        (3, "rating-star-3"),
        (4, "rating-star-4"),
        (5, "rating-star-5"),
    ] {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("star {n}"));
        assert!(
            bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
            "star {n} is {bounds:?}"
        );
    }
    assert!(
        cx.debug_bounds("rating-preview").is_none(),
        "no preview before the tap"
    );
    let star = cx.debug_bounds("rating-star-4").unwrap();
    let at = star.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_some(),
        "the press previews the rating"
    );
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_none(),
        "the release drops the preview"
    );
}

#[gpui::test]
fn a_touch_that_ends_outside_the_row_drops_the_preview(cx: &mut TestAppContext) {
    touch_theme(cx);
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Stars
    });
    cx.run_until_parked();
    let star = cx.debug_bounds("rating-star-4").unwrap();
    let at = star.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_some(),
        "the press previews the rating"
    );
    // The finger slides off the row and lifts there.
    let away = point(px(1000.), px(700.));
    cx.simulate_mouse_move(away, None, Modifiers::none());
    cx.simulate_mouse_up(away, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_none(),
        "a release outside the row drops the preview"
    );
}

#[gpui::test]
fn a_pointer_previews_a_star_and_leaves_the_preview_when_it_goes_away(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = false;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Stars
    });
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_none(),
        "no preview before the pointer arrives"
    );
    let star = cx.debug_bounds("rating-star-3").unwrap();
    let at = star.center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_some(),
        "the pointer previews the rating"
    );
    // The pointer leaves the row entirely.
    let away = point(px(1000.), px(700.));
    cx.simulate_mouse_move(away, None, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_none(),
        "leaving the row drops the preview"
    );
}

#[gpui::test]
fn a_pointer_keeps_the_preview_it_clicked(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = false;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Stars
    });
    cx.run_until_parked();
    let star = cx.debug_bounds("rating-star-4").unwrap();
    let at = star.center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_some(),
        "the click keeps the preview"
    );
    // The pointer leaves the row, but the click it made stays on screen.
    let away = point(px(1000.), px(700.));
    cx.simulate_mouse_move(away, None, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("rating-preview").is_some(),
        "the clicked rating outlives the pointer"
    );
}

struct Tree;
impl Render for Tree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(
            Cascader::new(
                "cascader",
                [
                    Cascade::new("asia", "Asia")
                        .child(Cascade::new("jp", "Japan").child(Cascade::new("tyo", "Tokyo"))),
                    Cascade::new("eu", "Europe"),
                ],
            )
            .on_change(|_, _, _| {}),
        )
    }
}

#[gpui::test]
fn a_tap_on_a_cascade_parent_opens_the_next_level_in_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Tree
    });
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_mouse_down(field.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(field.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("cascade-column-0").is_some(),
        "the first level is open"
    );
    assert!(
        cx.debug_bounds("cascade-column-2").is_none(),
        "the third level is still closed"
    );
    let parent = cx.debug_bounds("cascade-row-1-0").unwrap();
    let at = parent.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("cascade-column-2").is_some(),
        "the tap opens the next level"
    );
}
