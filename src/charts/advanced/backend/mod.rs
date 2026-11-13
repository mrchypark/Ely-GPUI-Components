//! GPUI backend for interactive high-throughput plots.

#![allow(clippy::collapsible_if)]

mod config;
mod constants;
mod frame;
mod geometry;
mod hover;
mod link;
mod paint;
mod state;
mod text;
mod view;

#[cfg(test)]
mod tests;

pub use config::PlotViewConfig;
pub use link::{LinkMemberId, PlotLinkGroup, PlotLinkOptions};
pub use view::{PlotEvent, PlotHandle, PlotHandlers, PlotView};
