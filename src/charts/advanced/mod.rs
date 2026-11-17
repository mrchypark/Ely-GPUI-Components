//! High-throughput interactive plotting with viewport-aware min-max decimation.

pub mod axis;
pub mod backend;
pub mod datasource;
pub mod element;
pub mod geom;
pub mod interaction;
pub mod plot;
pub mod render;
pub mod series;
pub mod style;
#[cfg(all(test, feature = "test-support"))]
mod tests;
pub mod transform;
pub mod view;

pub use axis::{AxisConfig, AxisConfigBuilder, AxisFormatter, AxisScale, TickConfig};
pub use backend::{
    LinkMemberId, PlotEvent, PlotHandle, PlotHandlers, PlotLinkGroup, PlotLinkOptions, PlotView,
    PlotViewConfig,
};
pub use datasource::AppendError;
pub use element::AdvancedChart;
pub use geom::Point;
pub use gpui::{Hsla, Rgba};
pub use interaction::Pin;
pub use plot::{Plot, PlotBuilder};
pub use render::{LineStyle, MarkerShape, MarkerStyle};
pub use series::{Series, SeriesId, SeriesKind};
pub use style::{Theme, Theme as PlotTheme, series_color};
pub use view::{Range, View, Viewport};
