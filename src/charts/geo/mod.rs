use std::{
    f64::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, ContentMask, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    PathBuilder, PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas,
    div,
};

use super::{
    axes::anchored,
    paint::{at, finish, measure, ring, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::compact,
    series::drawable,
    tiles::key,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize},
};

mod chart;
mod projection;
mod render;
mod scene;
mod state;
#[cfg(test)]
mod tests;
mod types;
mod util;

pub use chart::GeoChart;
pub use types::{GeoPoint, GeoProjection, GeoRegion};

#[allow(unused_imports)]
use {chart::*, projection::*, render::*, scene::*, state::*, types::*, util::*};
