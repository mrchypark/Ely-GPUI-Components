use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::{open, settle};
use crate::{
    documents::blocks::{BlockData, BlockEditor, BlockKind, Media},
    theme::Theme,
};

/// The editor, opened and then put in touch mode, since opening resets the theme.
fn open_touch(
    blocks: Vec<BlockData>,
    cx: &mut TestAppContext,
) -> (Entity<BlockEditor>, &mut VisualTestContext) {
    let (editor, cx) = open(blocks, cx);
    cx.update(|_window, cx| {
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    settle(cx);
    (editor, cx)
}

#[gpui::test]
fn a_touch_block_shows_its_gutter_without_a_hover(cx: &mut TestAppContext) {
    let (_, cx) = open_touch(vec![BlockData::new(BlockKind::Paragraph, ["lift"])], cx);
    let gutter = cx
        .debug_bounds("block-gutter-touch")
        .expect("the gutter shows");
    assert!(
        gutter.size.width >= gpui::px(48.) && gutter.size.height >= gpui::px(48.),
        "the gutter is {gutter:?}"
    );
}

#[gpui::test]
fn a_touch_picture_shows_its_align_tools_without_a_hover(cx: &mut TestAppContext) {
    let image = BlockData::new(BlockKind::Image(Media::new("missing.png")), [""]);
    let (_, cx) = open_touch(vec![image], cx);
    assert!(
        cx.debug_bounds("image-align-tools-touch").is_some(),
        "the align tools show"
    );
    assert!(
        cx.debug_bounds("image-align-button").is_some(),
        "the align buttons show"
    );
}

#[gpui::test]
fn a_tap_on_a_picture_reveals_its_align_tools_outside_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    // A picture that loads, so the press lands on it.
    let image = BlockData::new(BlockKind::Image(Media::new("logo.png")), [""]);
    let (_, cx) = open(vec![image], cx);
    settle(cx);
    assert!(
        cx.debug_bounds("image-align-tools-revealed").is_none(),
        "the tools hide until the picture is pressed"
    );
    let picture = cx.debug_bounds("image-picture").expect("the picture draws");
    let at = picture.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("image-align-tools-revealed").is_some(),
        "the tap reveals the align tools"
    );
}
