mod blocks;
mod counts;
mod epub;
mod find;
mod format;
mod history;
mod knowledge;
mod link;
mod markdown;
mod modes;
mod pages;
mod print;
mod reading;
mod render;
mod rich;
mod suggest;
mod templates;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod thumbs;
mod todo;
mod toolbar;
mod tree;
mod viewer;

use crate::theme::ActiveTheme;
use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, Styled, Window, div, prelude::FluentBuilder,
};

pub(crate) use blocks::source;
pub use blocks::{Align, BlockData, BlockEditor, BlockEvent, BlockKind, Media};
pub use counts::{ReadingTime, WordCount};
pub use epub::{Chapter, EpubReader};
pub use format::{Format, format};
pub(crate) use history::Change;
pub use history::{PageHistoryDiff, Version, VersionHistory};
pub use knowledge::{Backlink, Backlinks, Favorite, Favorites, TrashBin, Trashed};
pub use link::LinkEditor;
pub use markdown::markdown_highlights;
pub use modes::{MarkdownEditor, MarkdownMode};
pub use pages::{PageCover, PageIcon};
pub use print::{Margins, Paper, PrintPreview, PrintSettings, sheet};
pub use reading::{Footnote, Glossary, ReadingProgress, TooltipTerm, ZenMode};
pub use render::{MarkdownRenderer, outline};
pub use rich::RichTextEditor;
pub use templates::{Template, TemplatePicker};
pub use thumbs::PageThumbnailList;
pub use todo::{Checklist, TodoItem};
pub use toolbar::FixedFormatToolbar;
pub(crate) use viewer::step as zoom_step;
pub use viewer::{DocPage, DocumentViewer, PageHit, PageNote};

/// Binds the documents keys. `init` calls it.
pub(crate) fn bind_keys(cx: &mut App) {
    rich::bind_keys(cx);
}

/// Side-by-side on wide screens, stacked vertically on compact touch phones.
pub(super) fn sidebar_split(
    side: Option<AnyElement>,
    main: impl IntoElement,
    side_first: bool,
    window: &Window,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let compact = theme.touch_mode && window.viewport_size().width < theme.touch_compact_width();
    let border = theme.colors.border;
    let side_width = theme.sidebar_width(false);
    let side = side.map(|inner| {
        let panel = div().flex().flex_col().overflow_hidden();
        let panel = if compact {
            panel.w_full().flex_none().max_h(side_width)
        } else {
            panel.flex_none().w(side_width).h_full()
        };
        let panel = match (compact, side_first) {
            (true, true) => panel.border_b_1(),
            (true, false) => panel.border_t_1(),
            (false, true) => panel.border_r_1(),
            (false, false) => panel.border_l_1(),
        };
        panel.border_color(border).child(inner)
    });
    let main = div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .when(compact, |el| el.w_full())
        .child(main);
    let root = div().flex().when(compact, |el| el.flex_col());
    if side_first {
        root.children(side).child(main)
    } else {
        root.child(main).children(side)
    }
}
