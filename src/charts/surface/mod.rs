use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas, div, fill,
    prelude::*,
};

use super::{
    axes::{anchored, below},
    geometry::{Rect, frame},
    paint::{at, finish, measure, place, tint},
    parts::{ChartLegend, ChartTooltip},
    plot::Format,
    scale::compact,
    series::drawable,
    tiles::key,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, Mix, Radius, TextSize},
    typography::format as fmt,
};

mod chart;
mod render;
mod scene;
mod sets;
mod sets_render;
#[cfg(test)]
mod tests;
mod util;

pub use chart::SurfaceChart;
pub use sets::ParallelSetsChart;

#[allow(unused_imports)]
use {chart::*, render::*, scene::*, sets::*, sets_render::*, util::*};
