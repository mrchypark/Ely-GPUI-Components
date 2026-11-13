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

fn local_pos(position: gpui::Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

fn wrap_deg(deg: f32) -> f32 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

fn svg_hex(color: Hsla) -> (String, f32) {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(rgba.r),
            byte(rgba.g),
            byte(rgba.b)
        ),
        rgba.a,
    )
}

fn svg_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Isometric / perspective 3D surface mesh plot for 2D scalar grids z = f(x, y).
#[derive(IntoElement)]
pub struct SurfaceChart {
    base: Div,
    id: ElementId,
    cols: usize,
    rows: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
    values: Vec<f64>,
    azimuth_deg: f32,
    elevation_deg: f32,
    wireframe: bool,
    format: Format,
}

impl SurfaceChart {
    /// Build a surface mesh from a row-major cols * rows scalar grid.
    pub fn from_grid(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            cols * rows,
            "surface grid values length must equal cols * rows"
        );
        assert!(
            values.iter().all(|v| drawable(*v)),
            "surface grid values must be finite and within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            cols,
            rows,
            x_range: (0.0, cols.saturating_sub(1) as f64),
            y_range: (0.0, rows.saturating_sub(1) as f64),
            values,
            azimuth_deg: -38.0,
            elevation_deg: 28.0,
            wireframe: true,
            format: Rc::new(compact),
        }
    }

    /// Sample a 2D function f(x, y) across cols * rows points over x_range and y_range.
    pub fn from_fn(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
        f: impl Fn(f64, f64) -> f64,
    ) -> Self {
        assert!(
            drawable(x_range.0)
                && drawable(x_range.1)
                && drawable(y_range.0)
                && drawable(y_range.1),
            "surface ranges must be finite and within charts::LIMIT"
        );
        let mut values = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let ty = if rows > 1 {
                r as f64 / (rows - 1) as f64
            } else {
                0.5
            };
            let y = y_range.0 + (y_range.1 - y_range.0) * ty;
            for c in 0..cols {
                let tx = if cols > 1 {
                    c as f64 / (cols - 1) as f64
                } else {
                    0.5
                };
                let x = x_range.0 + (x_range.1 - x_range.0) * tx;
                let z = f(x, y);
                assert!(
                    drawable(z),
                    "surface function values must be finite and within charts::LIMIT"
                );
                values.push(z);
            }
        }
        Self {
            base: div(),
            id: id.into(),
            cols,
            rows,
            x_range,
            y_range,
            values,
            azimuth_deg: -38.0,
            elevation_deg: 28.0,
            wireframe: true,
            format: Rc::new(compact),
        }
    }

    /// Set camera azimuth and elevation angles in degrees (default (-38.0, 28.0)).
    pub fn angles(mut self, azimuth_deg: f32, elevation_deg: f32) -> Self {
        assert!(
            azimuth_deg.is_finite() && elevation_deg.is_finite(),
            "surface angles must be finite"
        );
        self.azimuth_deg = azimuth_deg;
        self.elevation_deg = elevation_deg;
        self
    }

    /// Toggle wireframe cell edges over surface quads (default true).
    pub fn wireframe(mut self, wireframe: bool) -> Self {
        self.wireframe = wireframe;
        self
    }

    /// Format numeric z values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the sampled grid as CSV (col,row,x,y,z).
    pub fn csv(&self) -> String {
        let mut out = String::from("col,row,x,y,z");
        for r in 0..self.rows {
            let ty = if self.rows > 1 {
                r as f64 / (self.rows - 1) as f64
            } else {
                0.5
            };
            let y = self.y_range.0 + (self.y_range.1 - self.y_range.0) * ty;
            for c in 0..self.cols {
                let tx = if self.cols > 1 {
                    c as f64 / (self.cols - 1) as f64
                } else {
                    0.5
                };
                let x = self.x_range.0 + (self.x_range.1 - self.x_range.0) * tx;
                let z = self.values[r * self.cols + c];
                write!(out, "\n{c},{r},{x},{y},{z}").expect("csv write");
            }
        }
        out
    }

    /// Export the 3D surface as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.inset),
            pixels(sizes.inset),
            pixels(sizes.inset),
        );
        let scene = build_surface_scene(
            self.cols,
            self.rows,
            &self.values,
            self.azimuth_deg,
            self.elevation_deg,
            1.0,
            rect,
        );
        let (bg_hex, _) = svg_hex(colors.bg);
        let (axis_hex, _) = svg_hex(colors.border);
        let (wire_hex, _) = svg_hex(colors.fg);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for &(a, b) in &scene.axes {
            write!(
                out,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{axis_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
                a.0, a.1, b.0, b.1
            )
            .expect("svg write");
        }
        for quad in &scene.quads {
            let fill = colors.chart[1].mix(&colors.chart[0], quad.t);
            let (fill_hex, _) = svg_hex(fill);
            let stroke_attr = if self.wireframe {
                format!(r#" stroke="{wire_hex}" stroke-opacity="0.22" stroke-width="0.75""#)
            } else {
                String::new()
            };
            write!(
                out,
                r#"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} Z" fill="{fill_hex}" fill-opacity="0.92"{stroke_attr}/>"#,
                quad.pts[0].0,
                quad.pts[0].1,
                quad.pts[1].0,
                quad.pts[1].1,
                quad.pts[2].0,
                quad.pts[2].1,
                quad.pts[3].0,
                quad.pts[3].1,
            )
            .expect("svg write");
        }
        for ((lx, ly), label) in &scene.axis_labels {
            write!(
                out,
                r#"<text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                svg_escape(label)
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

#[derive(Clone, Debug)]
struct SurfaceQuad {
    col: usize,
    row: usize,
    z: f64,
    t: f32,
    depth: f32,
    pts: [(f32, f32); 4],
    center: (f32, f32),
}

struct SurfaceScene {
    quads: Vec<SurfaceQuad>,
    axes: Vec<((f32, f32), (f32, f32))>,
    axis_labels: Vec<((f32, f32), SharedString)>,
}

fn project_raw(nx: f32, ny: f32, nz: f32, az: f32, el: f32) -> (f32, f32, f32) {
    let (sin_az, cos_az) = az.sin_cos();
    let (sin_el, cos_el) = el.sin_cos();
    let rx = nx * cos_az - ny * sin_az;
    let ry = nx * sin_az + ny * cos_az;
    let depth = ry * cos_el - nz * sin_el;
    let up = ry * sin_el + nz * cos_el;
    let persp = 3.6 / (3.6 + depth);
    (rx * persp, -up * persp, depth)
}

fn build_surface_scene(
    cols: usize,
    rows: usize,
    values: &[f64],
    azimuth_deg: f32,
    elevation_deg: f32,
    zoom: f32,
    rect: Rect,
) -> SurfaceScene {
    if cols < 2 || rows < 2 || rect.w <= 1.0 || rect.h <= 1.0 || values.len() != cols * rows {
        return SurfaceScene {
            quads: Vec::new(),
            axes: Vec::new(),
            axis_labels: Vec::new(),
        };
    }
    let (z_min, z_max) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let half_span = z_max * 0.5 - z_min * 0.5;
    let norm_z = |v: f64| -> f32 {
        if half_span > 0.0 {
            (((v * 0.5 - z_min * 0.5) / half_span) as f32).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };
    let (az, el) = (azimuth_deg.to_radians(), elevation_deg.to_radians());

    // Fit the normalized [-0.5, 0.5]^3 bounding cube into rect.
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for &cx in &[-0.5_f32, 0.5] {
        for &cy in &[-0.5_f32, 0.5] {
            for &cz in &[-0.5_f32, 0.5] {
                let (px, py, _) = project_raw(cx, cy, cz, az, el);
                min_x = min_x.min(px);
                max_x = max_x.max(px);
                min_y = min_y.min(py);
                max_y = max_y.max(py);
            }
        }
    }
    let span_x = (max_x - min_x).max(1e-4);
    let span_y = (max_y - min_y).max(1e-4);
    let scale = (rect.w / span_x).min(rect.h / span_y) * 0.82 * zoom.clamp(0.5, 4.0);
    let offset_x = rect.x + rect.w * 0.5 - (min_x + max_x) * 0.5 * scale;
    let offset_y = rect.y + rect.h * 0.5 - (min_y + max_y) * 0.5 * scale;
    let to_screen = |nx: f32, ny: f32, nz: f32| -> ((f32, f32), f32) {
        let (px, py, depth) = project_raw(nx, ny, nz, az, el);
        ((offset_x + px * scale, offset_y + py * scale), depth)
    };

    let mut projected = Vec::with_capacity(cols * rows);
    let mut heights = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let ny = r as f32 / (rows - 1) as f32 - 0.5;
        for c in 0..cols {
            let nx = c as f32 / (cols - 1) as f32 - 0.5;
            let t = norm_z(values[r * cols + c]);
            heights.push(t);
            projected.push(to_screen(nx, ny, t - 0.5));
        }
    }

    let mut quads = Vec::with_capacity((cols - 1) * (rows - 1));
    for r in 0..rows - 1 {
        for c in 0..cols - 1 {
            let i00 = r * cols + c;
            let i10 = r * cols + c + 1;
            let i11 = (r + 1) * cols + c + 1;
            let i01 = (r + 1) * cols + c;
            let (p00, d00) = projected[i00];
            let (p10, d10) = projected[i10];
            let (p11, d11) = projected[i11];
            let (p01, d01) = projected[i01];
            let depth = (d00 + d10 + d11 + d01) * 0.25;
            let t = (heights[i00] + heights[i10] + heights[i11] + heights[i01]) * 0.25;
            let z = (values[i00] * 0.25 + values[i10] * 0.25)
                + (values[i11] * 0.25 + values[i01] * 0.25);
            let center = (
                (p00.0 + p10.0 + p11.0 + p01.0) * 0.25,
                (p00.1 + p10.1 + p11.1 + p01.1) * 0.25,
            );
            quads.push(SurfaceQuad {
                col: c,
                row: r,
                z,
                t,
                depth,
                pts: [p00, p10, p11, p01],
                center,
            });
        }
    }
    // Painter's algorithm: farthest depth drawn first.
    quads.sort_by(|a, b| b.depth.total_cmp(&a.depth));

    // 3D bounding base frame on z = -0.5 plus vertical Z axis at the backmost base corner.
    let base_corners = [(-0.5_f32, -0.5_f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let base_proj: Vec<((f32, f32), f32)> = base_corners
        .iter()
        .map(|&(bx, by)| to_screen(bx, by, -0.5))
        .collect();
    let mut axes = vec![
        (base_proj[0].0, base_proj[1].0),
        (base_proj[1].0, base_proj[2].0),
        (base_proj[2].0, base_proj[3].0),
        (base_proj[3].0, base_proj[0].0),
    ];
    let back_ix = base_proj
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
        .map(|(ix, _)| ix)
        .unwrap_or(0);
    let (bx, by) = base_corners[back_ix];
    let (z_top, _) = to_screen(bx, by, 0.5);
    axes.push((base_proj[back_ix].0, z_top));

    let (x_mid, _) = to_screen(0.0, -0.58, -0.5);
    let (y_mid, _) = to_screen(0.58, 0.0, -0.5);
    let (z_lbl, _) = to_screen(bx, by, 0.57);
    let axis_labels = vec![
        (x_mid, SharedString::from("X")),
        (y_mid, SharedString::from("Y")),
        (z_lbl, SharedString::from("Z")),
    ];

    SurfaceScene {
        quads,
        axes,
        axis_labels,
    }
}

fn point_in_quad((px, py): (f32, f32), pts: [(f32, f32); 4]) -> bool {
    let mut inside = false;
    let mut j = 3;
    for i in 0..4 {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        let crosses = (yi > py) != (yj > py);
        if crosses {
            let denom = yj - yi;
            if denom.abs() > 1e-6 && px < (xj - xi) * (py - yi) / denom + xi {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

struct SurfaceState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    azimuth_offset: f32,
    elevation_offset: f32,
    zoom: f32,
    drag_last: Option<(f32, f32)>,
}

impl Default for SurfaceState {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            hover: None,
            azimuth_offset: 0.0,
            elevation_offset: 0.0,
            zoom: 1.0,
            drag_last: None,
        }
    }
}

impl Styled for SurfaceChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SurfaceChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<SurfaceState> =
            window.use_keyed_state((self.id.clone(), "surface"), cx, |_, _| {
                SurfaceState::default()
            });
        let (bounds, hover_raw, az_off, el_off, zoom) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.azimuth_offset,
                st.elevation_offset,
                st.zoom,
            )
        };
        let azimuth_deg = (self.azimuth_deg + az_off).rem_euclid(360.0);
        let elevation_deg = (self.elevation_deg + el_off).clamp(-80.0, 80.0);
        let zoom = zoom.clamp(0.5, 4.0);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.inset),
            pixels(sizes.inset),
            pixels(sizes.inset),
        );
        let scene = build_surface_scene(
            self.cols,
            self.rows,
            &self.values,
            azimuth_deg,
            elevation_deg,
            zoom,
            rect,
        );
        let hover = hover_raw.filter(|&ix| ix < scene.quads.len());
        let format = self.format.clone();
        let low_color = colors.chart[1];
        let high_color = colors.chart[0];
        let steps = [0.0, 0.25, 0.5, 0.75, 1.0].map(|t| low_color.mix(&high_color, t));
        let legend = key(steps, ("Low", "High"), cx);

        let tooltip = hover.map(|ix| {
            let quad = &scene.quads[ix];
            let fill_color = low_color.mix(&high_color, quad.t);
            let card = ChartTooltip::new(format!("({}, {})", quad.col, quad.row))
                .row(Some(fill_color), "Z", format(quad.z))
                .row(None, "Col", quad.col.to_string())
                .row(None, "Row", quad.row.to_string());
            anchored(quad.center, quad.center.0 > box_size.0 * 0.55, card)
        });

        let labels: Vec<Div> = scene
            .axis_labels
            .iter()
            .map(|((lx, ly), text)| {
                div()
                    .absolute()
                    .left(Pixels::from(*lx))
                    .top(Pixels::from(*ly))
                    .w_0()
                    .h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex_none()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(text.clone()),
                    )
            })
            .collect();

        let hit_quads: Vec<[(f32, f32); 4]> = scene.quads.iter().map(|q| q.pts).collect();
        let hit = Rc::new(move |pos: (f32, f32)| {
            // Scan front-to-back so the visually topmost quad wins.
            hit_quads
                .iter()
                .enumerate()
                .rev()
                .find(|(_, pts)| point_in_quad(pos, **pts))
                .map(|(ix, _)| ix)
        });
        let paint_quads = scene.quads;
        let paint_axes = scene.axes;
        let wireframe = self.wireframe;
        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let modified = az_off.abs() > 1e-3 || el_off.abs() > 1e-3 || (zoom - 1.0).abs() > 1e-3;
        let reset_btn = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "reset"), "Reset")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .debug_selector(|| "surface-reset".into())
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.azimuth_offset = 0.0;
                        st.elevation_offset = 0.0;
                        st.zoom = 1.0;
                        st.drag_last = None;
                        cx.notify();
                    })
                }))
        });
        let base_el = self.elevation_deg;
        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (press_hit, move_hit) = (hit.clone(), hit);

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let chart = div()
            .id((self.id.clone(), "surface"))
            .debug_selector(|| "surface-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |st, cx| {
                    if event.click_count >= 2 {
                        st.azimuth_offset = 0.0;
                        st.elevation_offset = 0.0;
                        st.zoom = 1.0;
                        st.drag_last = None;
                        st.hover = press_hit(pos);
                        cx.notify();
                        return;
                    }
                    st.drag_last = Some(pos);
                    st.hover = press_hit(pos);
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if let (Some(last), Some(MouseButton::Left)) =
                        (st.drag_last, event.pressed_button)
                    {
                        let dx = pos.0 - last.0;
                        let dy = pos.1 - last.1;
                        if dx.abs() > 0.01 || dy.abs() > 0.01 {
                            st.azimuth_offset = wrap_deg(st.azimuth_offset + dx * 0.5);
                            let next_el =
                                (base_el + st.elevation_offset + dy * 0.5).clamp(-80.0, 80.0);
                            st.elevation_offset = next_el - base_el;
                            st.drag_last = Some(pos);
                            st.hover = None;
                            cx.notify();
                        }
                        return;
                    }
                    st.drag_last = None;
                    let next = move_hit(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released.update(cx, |st, _| {
                    st.drag_last = None;
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released_out.update(cx, |st, _| {
                    st.drag_last = None;
                });
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if !dx.is_finite() || !dy.is_finite() {
                    return;
                }
                scrolled.update(cx, |st, cx| {
                    if ev.source == ScrollSource::Touch {
                        if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                            st.azimuth_offset = wrap_deg(st.azimuth_offset + dx * 0.5);
                            let next_el =
                                (base_el + st.elevation_offset + dy * 0.5).clamp(-80.0, 80.0);
                            st.elevation_offset = next_el - base_el;
                            st.hover = None;
                            cx.notify();
                        }
                    } else if dy.abs() >= 0.01 {
                        let factor = (1.0 - dy * 0.002).clamp(0.1, 10.0);
                        st.zoom = (st.zoom * factor).clamp(0.5, 4.0);
                        cx.notify();
                    }
                });
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                pinched.update(cx, |st, cx| {
                    st.drag_last = None;
                    if ev.phase != TouchPhase::Cancelled && ev.delta.is_finite() {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if (scale - 1.0).abs() > 1e-4 {
                            let next_zoom = (st.zoom * scale).clamp(0.5, 4.0);
                            if (next_zoom - st.zoom).abs() > f32::EPSILON {
                                st.zoom = next_zoom;
                                cx.notify();
                            }
                        }
                    }
                });
                cx.stop_propagation();
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        st.hover = None;
                        st.drag_last = None;
                        cx.notify();
                    });
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let axis_color = colors.border.opacity(0.75);
                        for &(a, b) in &paint_axes {
                            let mut path = PathBuilder::stroke(hairline);
                            path.move_to(at(origin, a));
                            path.line_to(at(origin, b));
                            finish(path, axis_color, window);
                        }
                        let wire_color = colors.fg.opacity(0.22);
                        for (ix, quad) in paint_quads.iter().enumerate() {
                            let is_hover = hover == Some(ix);
                            let fill_color = low_color
                                .mix(&high_color, quad.t)
                                .opacity(if is_hover { 1.0 } else { 0.92 });
                            let mut fill_path = PathBuilder::fill();
                            fill_path.move_to(at(origin, quad.pts[0]));
                            for &pt in &quad.pts[1..] {
                                fill_path.line_to(at(origin, pt));
                            }
                            fill_path.close();
                            finish(fill_path, fill_color, window);
                            if wireframe || is_hover {
                                let mut edge =
                                    PathBuilder::stroke(if is_hover { stroke } else { hairline });
                                edge.move_to(at(origin, quad.pts[0]));
                                for &pt in &quad.pts[1..] {
                                    edge.line_to(at(origin, pt));
                                }
                                edge.close();
                                finish(edge, if is_hover { colors.fg } else { wire_color }, window);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels)
            .children(reset_btn)
            .children(tooltip)
            .child(measure(state, |st| &mut st.bounds));

        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}

/// Categorical multi-dimension alluvial / parallel sets diagram.
#[derive(IntoElement)]
pub struct ParallelSetsChart {
    base: Div,
    id: ElementId,
    dimensions: Vec<SharedString>,
    ribbons: Vec<(Vec<SharedString>, f64)>,
    format: Format,
}

impl ParallelSetsChart {
    /// Create a parallel sets diagram across the ordered dimensions.
    pub fn new(
        id: impl Into<ElementId>,
        dimensions: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            dimensions: dimensions.into_iter().map(Into::into).collect(),
            ribbons: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// Add a categorical path across every dimension carrying count.
    pub fn ribbon(
        mut self,
        path_categories: impl IntoIterator<Item = impl Into<SharedString>>,
        count: f64,
    ) -> Self {
        let path: Vec<SharedString> = path_categories.into_iter().map(Into::into).collect();
        assert_eq!(
            path.len(),
            self.dimensions.len(),
            "a parallel sets ribbon needs one category per dimension"
        );
        assert!(
            drawable(count) && count >= 0.0,
            "a parallel sets ribbon needs a non-negative finite count within charts::LIMIT"
        );
        if count > 0.0 {
            self.ribbons.push((path, count));
        }
        self
    }

    /// Format numeric counts in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the parallel sets paths as CSV.
    pub fn csv(&self) -> String {
        let mut head: Vec<String> = self.dimensions.iter().map(|d| csv_field(d)).collect();
        head.push("count".to_string());
        let mut out = head.join(",");
        for (path, count) in &self.ribbons {
            out.push('\n');
            for cat in path {
                out.push_str(&csv_field(cat));
                out.push(',');
            }
            write!(out, "{count}").expect("csv write");
        }
        out
    }

    /// Export the parallel sets diagram as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.inset),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let bar_w = (pixels(sizes.inset) * 1.4).max(8.0);
        let layout = compute_parallel_sets(&self.dimensions, &self.ribbons, rect, bar_w);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (bar_hex, _) = svg_hex(colors.fg_muted);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for seg in &layout.segments {
            let (hex, _) = svg_hex(tint(colors, seg.color_ix));
            let mid = (seg.from.0 + seg.to.0) * 0.5;
            write!(
                out,
                r#"<path d="M{:.1},{:.1} C{mid:.1},{:.1} {mid:.1},{:.1} {:.1},{:.1} L{:.1},{:.1} C{mid:.1},{:.1} {mid:.1},{:.1} {:.1},{:.1} Z" fill="{hex}" fill-opacity="0.45"/>"#,
                seg.from.0,
                seg.from.1,
                seg.from.1,
                seg.to.1,
                seg.to.0,
                seg.to.1,
                seg.to.0,
                seg.to.1 + seg.thick,
                seg.to.1 + seg.thick,
                seg.from.1 + seg.thick,
                seg.from.0,
                seg.from.1 + seg.thick,
            )
            .expect("svg write");
        }
        for bar in &layout.bars {
            let (hex, opacity) = if bar.dim == 0 {
                (svg_hex(tint(colors, bar.cat_ix)).0, 0.9)
            } else {
                (bar_hex.clone(), 0.45)
            };
            write!(
                out,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{hex}" fill-opacity="{opacity}"/>"#,
                bar.rect.x, bar.rect.y, bar.rect.w, bar.rect.h
            )
            .expect("svg write");
        }
        for (dim, x) in layout.dim_xs.iter().enumerate() {
            write!(
                out,
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                rect.y + rect.h + 16.0,
                svg_escape(&self.dimensions[dim])
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

#[derive(Clone, Debug)]
struct SetBar {
    dim: usize,
    cat_ix: usize,
    name: SharedString,
    rect: Rect,
    count: f64,
}

#[derive(Clone, Copy, Debug)]
struct SetSegment {
    ribbon_ix: usize,
    color_ix: usize,
    dim: usize,
    from_cat: usize,
    to_cat: usize,
    from: (f32, f32),
    to: (f32, f32),
    thick: f32,
}

impl SetSegment {
    fn top(&self, t: f32) -> (f32, f32) {
        let mid = (self.from.0 + self.to.0) * 0.5;
        let u = 1.0 - t;
        let x = u * u * u * self.from.0
            + 3.0 * u * u * t * mid
            + 3.0 * u * t * t * mid
            + t * t * t * self.to.0;
        let y = u * u * u * self.from.1
            + 3.0 * u * u * t * self.from.1
            + 3.0 * u * t * t * self.to.1
            + t * t * t * self.to.1;
        (x, y)
    }

    fn holds(&self, (x, y): (f32, f32)) -> bool {
        if x < self.from.0 || x > self.to.0 || self.thick <= 0.0 {
            return false;
        }
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..24 {
            let mid = (lo + hi) * 0.5;
            if self.top(mid).0 < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let top_y = self.top(lo).1;
        y >= top_y && y <= top_y + self.thick
    }
}

struct ParallelSetsLayout {
    bars: Vec<SetBar>,
    segments: Vec<SetSegment>,
    ribbon_anchors: Vec<(f32, f32)>,
    dim_xs: Vec<f32>,
    total: f64,
}

fn compute_parallel_sets(
    dimensions: &[SharedString],
    ribbons: &[(Vec<SharedString>, f64)],
    rect: Rect,
    bar_w: f32,
) -> ParallelSetsLayout {
    let d_count = dimensions.len();
    let total: f64 = ribbons.iter().map(|(_, c)| *c).sum();
    if d_count == 0 || ribbons.is_empty() || total <= 0.0 || rect.w <= bar_w || rect.h <= 1.0 {
        return ParallelSetsLayout {
            bars: Vec::new(),
            segments: Vec::new(),
            ribbon_anchors: Vec::new(),
            dim_xs: Vec::new(),
            total,
        };
    }

    // Collect ordered categories and totals per dimension.
    let mut dim_cats: Vec<Vec<(SharedString, f64)>> = vec![Vec::new(); d_count];
    let mut dim_index: Vec<HashMap<SharedString, usize>> = vec![HashMap::new(); d_count];
    let mut ribbon_cats: Vec<Vec<usize>> = Vec::with_capacity(ribbons.len());
    for (path, count) in ribbons {
        let mut row_cats = Vec::with_capacity(d_count);
        for (d, name) in path.iter().enumerate() {
            let cat_ix = if let Some(&ix) = dim_index[d].get(name) {
                dim_cats[d][ix].1 += *count;
                ix
            } else {
                let ix = dim_cats[d].len();
                dim_index[d].insert(name.clone(), ix);
                dim_cats[d].push((name.clone(), *count));
                ix
            };
            row_cats.push(cat_ix);
        }
        ribbon_cats.push(row_cats);
    }

    let max_cats = dim_cats.iter().map(Vec::len).max().unwrap_or(1);
    let gap = if max_cats > 1 {
        8.0_f32.min((rect.h * 0.42) / (max_cats - 1) as f32)
    } else {
        0.0
    };
    let min_avail_h = dim_cats
        .iter()
        .map(|cats| (rect.h - gap * cats.len().saturating_sub(1) as f32).max(1.0))
        .fold(f32::INFINITY, f32::min);
    let height_of = |count: f64| ((count / total) as f32) * min_avail_h;

    let dim_xs: Vec<f32> = (0..d_count)
        .map(|d| {
            if d_count > 1 {
                rect.x + bar_w * 0.5 + (rect.w - bar_w) * d as f32 / (d_count - 1) as f32
            } else {
                rect.x + rect.w * 0.5
            }
        })
        .collect();

    let mut bars = Vec::new();
    let mut bar_lookup: Vec<Vec<Rect>> = vec![Vec::new(); d_count];
    for (d, cats) in dim_cats.iter().enumerate() {
        let col_h: f32 = cats.iter().map(|(_, c)| height_of(*c)).sum::<f32>()
            + gap * cats.len().saturating_sub(1) as f32;
        let mut y = rect.y + (rect.h - col_h) * 0.5;
        let x = dim_xs[d] - bar_w * 0.5;
        for (cat_ix, (name, count)) in cats.iter().enumerate() {
            let h = height_of(*count);
            let r = Rect { x, y, w: bar_w, h };
            bars.push(SetBar {
                dim: d,
                cat_ix,
                name: name.clone(),
                rect: r,
                count: *count,
            });
            bar_lookup[d].push(r);
            y += h + gap;
        }
    }

    // Build cubic S-curve segments between consecutive dimensions.
    let mut segments = Vec::with_capacity(ribbons.len() * d_count.saturating_sub(1));
    let mut ribbon_anchors = vec![(0.0_f32, 0.0_f32); ribbons.len()];
    for d in 0..d_count.saturating_sub(1) {
        let n_from = dim_cats[d].len();
        let n_to = dim_cats[d + 1].len();
        let mut out_offset = vec![0.0_f32; n_from];
        let mut in_offset = vec![0.0_f32; n_to];

        let mut order: Vec<usize> = (0..ribbons.len()).collect();
        order.sort_by_key(|&rix| {
            (
                ribbon_cats[rix][d],
                ribbon_cats[rix][d + 1],
                ribbon_cats[rix][0],
                rix,
            )
        });
        let mut from_ys = vec![0.0_f32; ribbons.len()];
        for &rix in &order {
            let fc = ribbon_cats[rix][d];
            let thick = height_of(ribbons[rix].1);
            from_ys[rix] = bar_lookup[d][fc].y + out_offset[fc];
            out_offset[fc] += thick;
        }

        order.sort_by_key(|&rix| {
            (
                ribbon_cats[rix][d + 1],
                ribbon_cats[rix][d],
                ribbon_cats[rix][0],
                rix,
            )
        });
        let mut to_ys = vec![0.0_f32; ribbons.len()];
        for &rix in &order {
            let tc = ribbon_cats[rix][d + 1];
            let thick = height_of(ribbons[rix].1);
            to_ys[rix] = bar_lookup[d + 1][tc].y + in_offset[tc];
            in_offset[tc] += thick;
        }

        for rix in 0..ribbons.len() {
            let fc = ribbon_cats[rix][d];
            let tc = ribbon_cats[rix][d + 1];
            let thick = height_of(ribbons[rix].1);
            let seg = SetSegment {
                ribbon_ix: rix,
                color_ix: ribbon_cats[rix][0],
                dim: d,
                from_cat: fc,
                to_cat: tc,
                from: (bar_lookup[d][fc].x + bar_w, from_ys[rix]),
                to: (bar_lookup[d + 1][tc].x, to_ys[rix]),
                thick,
            };
            if d == 0 {
                let mid = seg.top(0.5);
                ribbon_anchors[rix] = (mid.0, mid.1 + thick * 0.5);
            }
            segments.push(seg);
        }
    }
    if d_count == 1 {
        for (rix, cats) in ribbon_cats.iter().enumerate() {
            let r = bar_lookup[0][cats[0]];
            ribbon_anchors[rix] = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        }
    }

    ParallelSetsLayout {
        bars,
        segments,
        ribbon_anchors,
        dim_xs,
        total,
    }
}

#[derive(Default)]
struct ParSetsState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    pinned: Option<usize>,
    hidden: HashSet<SharedString>,
    press_pos: Option<(f32, f32)>,
}

impl Styled for ParallelSetsChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ParallelSetsChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<ParSetsState> =
            window.use_keyed_state((self.id.clone(), "parsets"), cx, |_, _| {
                ParSetsState::default()
            });
        let (bounds, raw_hover, raw_pinned, hidden) = {
            let st = state.read(cx);
            (st.bounds, st.hover, st.pinned, st.hidden.clone())
        };
        let mut first_cats: Vec<SharedString> = Vec::new();
        for (path, _) in &self.ribbons {
            if let Some(cat) = path.first()
                && !first_cats.contains(cat)
            {
                first_cats.push(cat.clone());
            }
        }
        let first_color_map: HashMap<SharedString, usize> = first_cats
            .iter()
            .enumerate()
            .map(|(ix, cat)| (cat.clone(), ix))
            .collect();
        let orig_count = self.ribbons.len();
        let mut visible_ribbons: Vec<(Vec<SharedString>, f64)> = Vec::new();
        let mut vis_to_orig: Vec<usize> = Vec::new();
        let mut orig_to_vis: Vec<Option<usize>> = vec![None; orig_count];
        let mut vis_color_ix: Vec<usize> = Vec::new();
        for (orig_ix, (path, count)) in self.ribbons.iter().enumerate() {
            if path.first().is_some_and(|cat| hidden.contains(cat)) {
                continue;
            }
            orig_to_vis[orig_ix] = Some(visible_ribbons.len());
            vis_to_orig.push(orig_ix);
            vis_color_ix.push(
                path.first()
                    .and_then(|cat| first_color_map.get(cat).copied())
                    .unwrap_or(0),
            );
            visible_ribbons.push((path.clone(), *count));
        }

        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.inset),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let bar_w = (pixels(sizes.inset) * 1.4).max(8.0);
        let layout = compute_parallel_sets(&self.dimensions, &visible_ribbons, rect, bar_w);
        let total_targets = orig_count + layout.bars.len();
        let active_pinned = raw_pinned.filter(|&ix| ix < orig_count && orig_to_vis[ix].is_some());
        let hover = raw_hover
            .filter(|&ix| ix < total_targets && (ix >= orig_count || orig_to_vis[ix].is_some()));
        let active = active_pinned.or(hover);
        let format = self.format.clone();

        let legend = (first_cats.len() > 1).then(|| {
            let toggled = state.clone();
            let legend = first_cats.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, name)| {
                    legend.entry(tint(&colors, ix), name.clone(), hidden.contains(name))
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |st, cx| {
                    if !st.hidden.remove(name) {
                        st.hidden.insert(name.clone());
                    }
                    st.pinned = None;
                    st.hover = None;
                    cx.notify();
                });
            })
        });

        let active_ribbon_vis =
            active.and_then(|ix| (ix < orig_count).then(|| orig_to_vis[ix]).flatten());
        let active_bar = active.and_then(|ix| ix.checked_sub(orig_count));

        let tooltip = active.map(|ix| {
            if ix < orig_count {
                let vis_ix = orig_to_vis[ix].expect("visible ribbon");
                let (path, count) = &self.ribbons[ix];
                let title = path
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .join(" › ");
                let share = if layout.total > 0.0 {
                    *count / layout.total
                } else {
                    0.0
                };
                let color_ix = vis_color_ix.get(vis_ix).copied().unwrap_or(0);
                let card = ChartTooltip::new(title)
                    .row(Some(tint(&colors, color_ix)), "Count", format(*count))
                    .row(None, "Share", fmt::percent(share, 1, false));
                let pos = layout.ribbon_anchors[vis_ix];
                anchored(pos, pos.0 > box_size.0 * 0.55, card)
            } else {
                let bar = &layout.bars[ix - orig_count];
                let share = if layout.total > 0.0 {
                    bar.count / layout.total
                } else {
                    0.0
                };
                let dot = (bar.dim == 0).then(|| {
                    let c_ix = first_color_map
                        .get(&bar.name)
                        .copied()
                        .unwrap_or(bar.cat_ix);
                    tint(&colors, c_ix)
                });
                let card = ChartTooltip::new(format!("{}: {}", self.dimensions[bar.dim], bar.name))
                    .row(dot, "Count", format(bar.count))
                    .row(None, "Share", fmt::percent(share, 1, false));
                let pos = (bar.rect.x + bar.rect.w, bar.rect.y + bar.rect.h * 0.5);
                anchored(pos, pos.0 > box_size.0 * 0.55, card)
            }
        });

        let dim_labels: Vec<Div> = layout
            .dim_xs
            .iter()
            .enumerate()
            .map(|(d, &x)| {
                below(
                    x,
                    rect,
                    div()
                        .flex_none()
                        .whitespace_nowrap()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(self.dimensions[d].clone()),
                )
            })
            .collect();

        let d_count = self.dimensions.len();
        let cat_labels: Vec<Div> = layout
            .bars
            .iter()
            .filter(|bar| bar.rect.h >= 12.0)
            .map(|bar| {
                let is_last = d_count > 1 && bar.dim == d_count - 1;
                let anchor_x = if is_last {
                    bar.rect.x - 4.0
                } else {
                    bar.rect.x + bar.rect.w + 4.0
                };
                div()
                    .absolute()
                    .left(Pixels::from(anchor_x))
                    .top(Pixels::from(bar.rect.y + bar.rect.h * 0.5))
                    .w_0()
                    .h_0()
                    .flex()
                    .items_center()
                    .when(is_last, |el| el.justify_end())
                    .child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .px_1()
                            .rounded(theme.radius(Radius::Sm))
                            .bg(colors.bg.opacity(0.82))
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg)
                            .child(bar.name.clone()),
                    )
            })
            .collect();

        let hit_bars: Vec<Rect> = layout.bars.iter().map(|b| b.rect).collect();
        let hit_segs = layout.segments.clone();
        let hit = Rc::new(move |pos: (f32, f32)| -> Option<usize> {
            if let Some(bar_ix) = hit_bars.iter().position(|r| r.contains(pos)) {
                return Some(orig_count + bar_ix);
            }
            hit_segs
                .iter()
                .find(|seg| seg.holds(pos))
                .and_then(|seg| vis_to_orig.get(seg.ribbon_ix).copied())
        });
        let paint_bars = layout.bars;
        let paint_segs = layout.segments;
        let corner = theme.radius(Radius::Sm).to_pixels(rem);
        let (moved, pressed, released, released_out, left) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (move_hit, press_hit, release_hit) = (hit.clone(), hit.clone(), hit);
        let has_active = active.is_some();
        let clear_btn = active_pinned.map(|_| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "clear"), "Clear")
                .debug_selector(|| "parallel-sets-clear".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.pinned = None;
                        st.hover = None;
                        st.press_pos = None;
                        cx.notify();
                    });
                });
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button)
        });

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "parsets"))
            .debug_selector(|| "parallel-sets-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                let next = move_hit(pos);
                moved.update(cx, |st, cx| {
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                let next = press_hit(pos);
                pressed.update(cx, |st, cx| {
                    st.press_pos = Some(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local_pos(event.position, released.read(cx).bounds);
                released.update(cx, |st, cx| {
                    let clicked = st
                        .press_pos
                        .take()
                        .is_some_and(|p| (pos.0 - p.0).hypot(pos.1 - p.1) <= 4.0);
                    if !clicked {
                        return;
                    }
                    let target = release_hit(pos);
                    st.hover = target;
                    if let Some(rix) = target.filter(|&ix| ix < orig_count) {
                        st.pinned = if st.pinned == Some(rix) {
                            None
                        } else {
                            Some(rix)
                        };
                    } else {
                        st.pinned = None;
                    }
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released_out.update(cx, |st, _| {
                    st.press_pos = None;
                });
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        if st.hover.take().is_some() {
                            cx.notify();
                        }
                    });
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let active_bar_ref = active_bar
                            .and_then(|ix| paint_bars.get(ix))
                            .map(|b| (b.dim, b.cat_ix));
                        for seg in &paint_segs {
                            let active = match (active_ribbon_vis, active_bar_ref) {
                                (Some(rix), _) => seg.ribbon_ix == rix,
                                (None, Some((dim, cat))) => {
                                    (seg.dim == dim && seg.from_cat == cat)
                                        || (seg.dim + 1 == dim && seg.to_cat == cat)
                                }
                                (None, None) => true,
                            };
                            let alpha = if !has_active {
                                0.4
                            } else if active {
                                0.74
                            } else {
                                0.1
                            };
                            let mut path = PathBuilder::fill();
                            path.move_to(at(origin, seg.from));
                            let mid = (seg.from.0 + seg.to.0) * 0.5;
                            path.cubic_bezier_to(
                                at(origin, seg.to),
                                at(origin, (mid, seg.from.1)),
                                at(origin, (mid, seg.to.1)),
                            );
                            path.line_to(at(origin, (seg.to.0, seg.to.1 + seg.thick)));
                            path.cubic_bezier_to(
                                at(origin, (seg.from.0, seg.from.1 + seg.thick)),
                                at(origin, (mid, seg.to.1 + seg.thick)),
                                at(origin, (mid, seg.from.1 + seg.thick)),
                            );
                            path.close();
                            let color_ix = vis_color_ix
                                .get(seg.ribbon_ix)
                                .copied()
                                .unwrap_or(seg.color_ix);
                            finish(path, tint(&colors, color_ix).opacity(alpha), window);
                        }
                        for (ix, bar) in paint_bars.iter().enumerate() {
                            let lit = active_bar == Some(ix);
                            let color = if bar.dim == 0 {
                                let c_ix = first_color_map
                                    .get(&bar.name)
                                    .copied()
                                    .unwrap_or(bar.cat_ix);
                                tint(&colors, c_ix).opacity(if lit { 1.0 } else { 0.88 })
                            } else {
                                colors.fg_muted.opacity(if lit { 0.8 } else { 0.45 })
                            };
                            window.paint_quad(
                                fill(place(origin, bar.rect), color).corner_radii(corner),
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(dim_labels)
            .children(cat_labels)
            .children(clear_btn)
            .children(tooltip)
            .child(measure(state, |st| &mut st.bounds));

        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_scene_sorts_quads_back_to_front_and_handles_degenerate_grids() {
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            w: 200.0,
            h: 160.0,
        };
        let empty = build_surface_scene(1, 3, &[1.0, 2.0, 3.0], -38.0, 28.0, 1.0, rect);
        assert!(empty.quads.is_empty());

        let flat = build_surface_scene(3, 3, &[5.0; 9], -38.0, 28.0, 1.0, rect);
        assert_eq!(flat.quads.len(), 4);
        assert!(flat.quads.iter().all(|q| (q.t - 0.5).abs() < 1e-6));

        let hill = SurfaceChart::from_fn("hill", 4, 4, (-1.0, 1.0), (-1.0, 1.0), |x, y| {
            1.0 - (x * x + y * y)
        });
        let scene = build_surface_scene(
            hill.cols,
            hill.rows,
            &hill.values,
            hill.azimuth_deg,
            hill.elevation_deg,
            1.0,
            rect,
        );
        assert_eq!(scene.quads.len(), 9);
        assert!(
            scene
                .quads
                .windows(2)
                .all(|pair| pair[0].depth >= pair[1].depth)
        );

        let zoomed = build_surface_scene(
            hill.cols,
            hill.rows,
            &hill.values,
            hill.azimuth_deg,
            hill.elevation_deg,
            2.0,
            rect,
        );
        assert!((zoomed.quads[0].center.0 - 110.0).abs() > (scene.quads[0].center.0 - 110.0).abs());
    }

    #[test]
    fn parallel_sets_stacks_bars_and_aligns_ribbon_thickness() {
        let dims = ["Class", "Sex", "Survived"].map(SharedString::from);
        let ribbons = vec![
            (vec!["1st".into(), "Female".into(), "Yes".into()], 30.0),
            (vec!["1st".into(), "Male".into(), "No".into()], 20.0),
            (vec!["3rd".into(), "Male".into(), "No".into()], 50.0),
        ];
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            w: 300.0,
            h: 160.0,
        };
        let layout = compute_parallel_sets(&dims, &ribbons, rect, 12.0);
        assert_eq!(layout.bars.len(), 6);
        assert_eq!(layout.segments.len(), 6);
        assert_eq!(layout.total, 100.0);
        // Segments of the same ribbon keep equal thickness across dimensions.
        let s0 = layout.segments[0];
        let s3 = layout.segments[3];
        assert!((s0.thick - s3.thick).abs() < 1e-4);
        let mid = s0.top(0.5);
        assert!(s0.holds((mid.0, mid.1 + s0.thick * 0.5)));
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn surface_and_parallel_sets_render_hover_and_export(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{
            Context, Modifiers, MouseButton, PinchEvent, Render, ScrollDelta, ScrollEvent,
            ScrollSource, TouchPhase, Window, point, px, size,
        };

        struct SurfaceHarness;
        impl Render for SurfaceHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).child(SurfaceChart::from_fn(
                    "surf",
                    5,
                    5,
                    (-1.0, 1.0),
                    (-1.0, 1.0),
                    |x, y| (x * 2.0).cos() + (y * 2.0).sin(),
                ))
            }
        }

        struct ParSetsHarness;
        impl Render for ParSetsHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).child(
                    ParallelSetsChart::new("titanic", ["Class", "Sex", "Survived"])
                        .ribbon(["1st", "Female", "Yes"], 40.0)
                        .ribbon(["1st", "Male", "No"], 20.0)
                        .ribbon(["3rd", "Male", "No"], 60.0),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            let surface = SurfaceChart::from_grid(
                "export-surf",
                3,
                3,
                [0.0, 1.0, 0.0, 1.0, 2.0, 1.0, 0.0, 1.0, 0.0],
            )
            .angles(-45.0, 30.0)
            .wireframe(true);
            let s_svg = surface.svg(320.0, 220.0, cx);
            assert!(s_svg.starts_with("<svg") && s_svg.contains("<path"));
            assert!(surface.csv().starts_with("col,row,x,y,z\n0,0,"));

            let parsets = ParallelSetsChart::new("export-ps", ["A", "B"])
                .ribbon(["X", "Y"], 10.0)
                .ribbon(["Z", "Y"], 15.0);
            let p_svg = parsets.svg(320.0, 220.0, cx);
            assert!(
                p_svg.starts_with("<svg") && p_svg.contains("<path") && p_svg.contains("<rect")
            );
            assert_eq!(parsets.csv(), "A,B,count\nX,Y,10\nZ,Y,15");
        });

        let (_, vc) = cx.add_window_view(|_, _| SurfaceHarness);
        vc.simulate_resize(size(px(400.0), px(320.0)));
        settle(vc);
        let surf_bounds = vc
            .debug_bounds("surface-plot")
            .expect("surface plot bounds");
        vc.simulate_mouse_move(surf_bounds.center(), None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "hovering surface center opens tooltip"
        );
        assert!(vc.debug_bounds("surface-reset").is_none());

        let start = surf_bounds.center();
        let dragged = point(start.x + px(48.0), start.y + px(24.0));
        vc.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(dragged, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(dragged, MouseButton::Left, Modifiers::none());
        settle(vc);
        let reset_bounds = vc
            .debug_bounds("surface-reset")
            .expect("reset button appears after drag orbit");

        vc.simulate_mouse_down(reset_bounds.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_bounds.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_none());

        vc.simulate_event(ScrollEvent {
            position: surf_bounds.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_some());

        vc.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: surf_bounds.center(),
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: surf_bounds.center(),
            modifiers: Modifiers::none(),
            click_count: 2,
        });
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_none());

        vc.simulate_event(ScrollEvent {
            position: surf_bounds.center(),
            delta: ScrollDelta::Pixels(point(px(30.0), px(-20.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_some());

        let reset_bounds = vc
            .debug_bounds("surface-reset")
            .expect("reset button appears after touch orbit");
        vc.simulate_mouse_down(reset_bounds.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_bounds.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_none());

        vc.simulate_event(PinchEvent {
            position: surf_bounds.center(),
            delta: 0.35,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_some());

        let (_, vc) = cx.add_window_view(|_, _| ParSetsHarness);
        vc.simulate_resize(size(px(400.0), px(320.0)));
        settle(vc);
        let ps_bounds = vc
            .debug_bounds("parallel-sets-plot")
            .expect("parallel sets plot bounds");
        vc.simulate_mouse_down(ps_bounds.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(ps_bounds.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "pressing parallel sets center opens tooltip"
        );

        let ribbon_pt = point(
            ps_bounds.left() + ps_bounds.size.width * 0.28,
            ps_bounds.top() + ps_bounds.size.height * 0.72,
        );
        let empty_pt = point(ps_bounds.left() + px(4.0), ps_bounds.top() + px(4.0));
        vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        vc.simulate_mouse_move(empty_pt, None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "pinned ribbon keeps tooltip visible when pointer moves away"
        );

        vc.simulate_mouse_down(empty_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(empty_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_none());

        vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());
        vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(empty_pt, None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_none());

        let legend_0 = vc
            .debug_bounds("chart-legend-entry-0")
            .expect("first-dim legend entry");
        vc.simulate_mouse_down(legend_0.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(legend_0.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        let legend_1 = vc
            .debug_bounds("chart-legend-entry-1")
            .expect("second first-dim legend entry");
        vc.simulate_mouse_down(legend_1.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(legend_1.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn surface_and_parallel_sets_touch_mode_pinch_clear_and_flip(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{
            Context, Modifiers, MouseButton, PinchEvent, Render, TouchPhase, Window, point, px,
            size,
        };

        struct SurfaceTouchHarness;
        impl Render for SurfaceTouchHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(260.0)).child(SurfaceChart::from_fn(
                    "surf-touch",
                    5,
                    5,
                    (-1.0, 1.0),
                    (-1.0, 1.0),
                    |x, y| (x * 2.0).cos() + (y * 2.0).sin(),
                ))
            }
        }

        struct ParSetsTouchHarness;
        impl Render for ParSetsTouchHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(260.0)).child(
                    ParallelSetsChart::new("ps-touch", ["Class", "Sex", "Survived"])
                        .ribbon(["1st", "Female", "Yes"], 40.0)
                        .ribbon(["1st", "Male", "No"], 20.0)
                        .ribbon(["3rd", "Male", "No"], 60.0),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });

        let (_, vc) = cx.add_window_view(|_, _| SurfaceTouchHarness);
        vc.simulate_resize(size(px(260.0), px(320.0)));
        settle(vc);
        let surf_bounds = vc
            .debug_bounds("surface-plot")
            .expect("surface plot bounds");
        let center = surf_bounds.center();

        // Pinching after mouse_down clears drag_last so subsequent move hovers rather than orbiting.
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        vc.simulate_event(PinchEvent {
            position: center,
            delta: 0.35,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        vc.simulate_mouse_move(center, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
        settle(vc);
        let reset_btn = vc
            .debug_bounds("surface-reset")
            .expect("surface reset after pinch");
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("surface-reset").is_none());

        let (_, vc) = cx.add_window_view(|_, _| ParSetsTouchHarness);
        vc.simulate_resize(size(px(260.0), px(320.0)));
        settle(vc);
        let ps_bounds = vc
            .debug_bounds("parallel-sets-plot")
            .expect("parallel sets plot bounds");
        let ribbon_pt = point(
            ps_bounds.left() + ps_bounds.size.width * 0.28,
            ps_bounds.top() + ps_bounds.size.height * 0.72,
        );
        vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        let clear_btn = vc
            .debug_bounds("parallel-sets-clear")
            .expect("clear button appears when ribbon is pinned");
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        vc.simulate_mouse_down(clear_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(clear_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("parallel-sets-clear").is_none());
        assert!(vc.debug_bounds("chart-tooltip").is_none());
    }
}
