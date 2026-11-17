use gpui::{App, Rgba};

use crate::theme::{ActiveTheme, Theme as ElyTheme};

/// Visual theme for plot-level elements such as axes, grid, and overlays.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub background: Rgba,
    pub axis: Rgba,
    pub grid_major: Rgba,
    pub grid_minor: Rgba,
    pub hover_bg: Rgba,
    pub hover_border: Rgba,
    pub pin_bg: Rgba,
    pub pin_border: Rgba,
    pub selection_fill: Rgba,
    pub selection_border: Rgba,
    pub legend_bg: Rgba,
    pub legend_border: Rgba,
}

impl Theme {
    /// Create the default theme (alias of dark).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme palette.
    pub fn light() -> Self {
        Self {
            background: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            axis: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            },
            grid_major: Rgba {
                r: 0.86,
                g: 0.86,
                b: 0.86,
                a: 1.0,
            },
            grid_minor: Rgba {
                r: 0.93,
                g: 0.93,
                b: 0.93,
                a: 1.0,
            },
            hover_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.9,
            },
            hover_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.8,
            },
            pin_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.92,
            },
            pin_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.8,
            },
            selection_fill: Rgba {
                r: 0.1,
                g: 0.4,
                b: 0.9,
                a: 0.15,
            },
            selection_border: Rgba {
                r: 0.1,
                g: 0.4,
                b: 0.9,
                a: 0.9,
            },
            legend_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.85,
            },
            legend_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.6,
            },
        }
    }

    /// Create a dark theme palette.
    pub fn dark() -> Self {
        Self {
            background: Rgba {
                r: 0.08,
                g: 0.08,
                b: 0.09,
                a: 1.0,
            },
            axis: Rgba {
                r: 0.85,
                g: 0.85,
                b: 0.85,
                a: 1.0,
            },
            grid_major: Rgba {
                r: 0.25,
                g: 0.25,
                b: 0.28,
                a: 1.0,
            },
            grid_minor: Rgba {
                r: 0.18,
                g: 0.18,
                b: 0.2,
                a: 1.0,
            },
            hover_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.92,
            },
            hover_border: Rgba {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 0.8,
            },
            pin_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.92,
            },
            pin_border: Rgba {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 0.85,
            },
            selection_fill: Rgba {
                r: 0.2,
                g: 0.5,
                b: 0.95,
                a: 0.18,
            },
            selection_border: Rgba {
                r: 0.3,
                g: 0.6,
                b: 1.0,
                a: 0.9,
            },
            legend_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.9,
            },
            legend_border: Rgba {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 0.7,
            },
        }
    }

    /// Build a plot theme from Ely's active theme tokens.
    pub fn from_ely(theme: &ElyTheme) -> Self {
        let colors = &theme.colors;
        Self {
            background: colors.bg.into(),
            axis: colors.fg_muted.into(),
            grid_major: colors.border.opacity(0.75).into(),
            grid_minor: colors.border.opacity(0.35).into(),
            hover_bg: colors.overlay.opacity(0.95).into(),
            hover_border: colors.border_strong.into(),
            pin_bg: colors.overlay.opacity(0.95).into(),
            pin_border: colors.border_strong.into(),
            selection_fill: colors.selection.into(),
            selection_border: colors.focus.into(),
            legend_bg: colors.overlay.opacity(0.92).into(),
            legend_border: colors.border.into(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Returns the Ely chart palette color at index as an Rgba for advanced series.
pub fn series_color(index: usize, cx: &App) -> Rgba {
    let palette = &cx.theme().colors.chart;
    palette[index % palette.len()].into()
}
