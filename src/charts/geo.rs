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

/// Geographic map projections supported by [`GeoChart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoProjection {
    #[default]
    Equirectangular,
    Mercator,
    Orthographic,
    Stereographic,
}

/// A geographic polygon region (country, province, or zone) with an associated scalar value.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoRegion {
    pub name: SharedString,
    pub value: f64,
    pub rings: Vec<Vec<(f64, f64)>>,
}

impl GeoRegion {
    /// Create a region from its name, scalar value, and exterior `(lon_deg, lat_deg)` ring.
    pub fn new(
        name: impl Into<SharedString>,
        value: f64,
        exterior_ring: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        assert!(
            drawable(value),
            "a geo region needs a finite value within charts::LIMIT"
        );
        let ring = validate_ring(exterior_ring);
        Self {
            name: name.into(),
            value,
            rings: if ring.is_empty() {
                Vec::new()
            } else {
                vec![ring]
            },
        }
    }

    /// Append an additional ring (an island or interior cutout) in `(lon_deg, lat_deg)`.
    pub fn ring(mut self, extra_ring: impl IntoIterator<Item = (f64, f64)>) -> Self {
        let ring = validate_ring(extra_ring);
        if !ring.is_empty() {
            self.rings.push(ring);
        }
        self
    }
}

/// A geographic point marker at `(lon_deg, lat_deg)` with a scalar value.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoPoint {
    pub name: SharedString,
    pub lon: f64,
    pub lat: f64,
    pub value: f64,
}

impl GeoPoint {
    /// Create a geographic point at `(lon_deg, lat_deg)` in `-180..=180`, `-90..=90`.
    pub fn new(name: impl Into<SharedString>, lon: f64, lat: f64, value: f64) -> Self {
        assert!(
            lon.is_finite()
                && (-180.0..=180.0).contains(&lon)
                && lat.is_finite()
                && (-90.0..=90.0).contains(&lat),
            "geo point coordinates must be within lon -180..=180 and lat -90..=90"
        );
        assert!(
            drawable(value),
            "a geo point needs a finite value within charts::LIMIT"
        );
        Self {
            name: name.into(),
            lon,
            lat,
            value,
        }
    }
}

/// Geographic map chart supporting choropleth regions, scaled point markers, graticules, and 3D rotation.
#[derive(IntoElement)]
pub struct GeoChart {
    base: Div,
    id: ElementId,
    projection: GeoProjection,
    rotate: (f64, f64),
    graticule_step: f64,
    show_graticule: bool,
    regions: Vec<GeoRegion>,
    points: Vec<GeoPoint>,
    format: Format,
}

impl GeoChart {
    /// Create a geographic chart with Equirectangular projection and 30° graticule enabled.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            projection: GeoProjection::Equirectangular,
            rotate: (0.0, 0.0),
            graticule_step: 30.0,
            show_graticule: true,
            regions: Vec::new(),
            points: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// Select the map projection.
    pub fn projection(mut self, projection: GeoProjection) -> Self {
        self.projection = projection;
        self
    }

    /// Apply 3D spherical rotation `(yaw_deg, pitch_deg)` before projecting.
    pub fn rotate(mut self, yaw_deg: f64, pitch_deg: f64) -> Self {
        assert!(
            yaw_deg.is_finite() && pitch_deg.is_finite(),
            "geo rotation angles must be finite"
        );
        self.rotate = (yaw_deg, pitch_deg);
        self
    }

    /// Set the graticule grid spacing in degrees and enable it.
    pub fn graticule(mut self, step_deg: f64) -> Self {
        assert!(
            step_deg.is_finite() && step_deg > 0.0 && step_deg <= 180.0,
            "graticule step must be finite in (0, 180]"
        );
        self.graticule_step = step_deg;
        self.show_graticule = true;
        self
    }

    /// Show or hide the latitude/longitude graticule.
    pub fn show_graticule(mut self, show: bool) -> Self {
        self.show_graticule = show;
        self
    }

    /// Add a choropleth region.
    pub fn region(mut self, region: GeoRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Add multiple choropleth regions.
    pub fn regions(mut self, regions: impl IntoIterator<Item = GeoRegion>) -> Self {
        self.regions.extend(regions);
        self
    }

    /// Add a point marker.
    pub fn point(mut self, point: GeoPoint) -> Self {
        self.points.push(point);
        self
    }

    /// Add multiple point markers.
    pub fn points(mut self, points: impl IntoIterator<Item = GeoPoint>) -> Self {
        self.points.extend(points);
        self
    }

    /// Format numeric values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export regions and points as CSV (`kind,name,lon,lat,value`).
    pub fn csv(&self) -> String {
        let mut out = String::from("kind,name,lon,lat,value");
        for region in &self.regions {
            let (lon, lat) = region
                .rings
                .first()
                .and_then(|ring| centroid_deg(ring))
                .unwrap_or((0.0, 0.0));
            write!(
                out,
                "\nregion,{},{lon:.4},{lat:.4},{}",
                csv_field(&region.name),
                region.value
            )
            .expect("csv write");
        }
        for pt in &self.points {
            write!(
                out,
                "\npoint,{},{},{},{}",
                csv_field(&pt.name),
                pt.lon,
                pt.lat,
                pt.value
            )
            .expect("csv write");
        }
        out
    }

    /// Export the projected map as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (colors, sizes, rem) = (&theme.colors, theme.chart(), theme.base_rem());
        let pad = f32::from(sizes.inset.to_pixels(rem));
        let scene = build_scene(
            self,
            (width, height),
            pad,
            GeoView {
                rotate: self.rotate,
                zoom: 1.0,
                pan: (0.0, 0.0),
            },
        );
        let bg_hex = svg_hex(colors.bg);
        let border_hex = svg_hex(colors.border);
        let region_hex = svg_hex(tint(colors, 0));
        let point_hex = svg_hex(if self.regions.is_empty() {
            tint(colors, 0)
        } else {
            tint(colors, 1)
        });
        let (low, high) = region_extent(&self.regions);

        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");

        if let Some((cx_px, cy_px, r_px)) = scene.globe {
            write!(
                out,
                r#"<circle cx="{cx_px:.1}" cy="{cy_px:.1}" r="{r_px:.1}" fill="none" stroke="{border_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
            )
            .expect("svg write");
        } else if scene.frame_w > 0.0 && scene.frame_h > 0.0 {
            write!(
                out,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="none" stroke="{border_hex}" stroke-opacity="0.55" stroke-width="1"/>"#,
                scene.frame_x, scene.frame_y, scene.frame_w, scene.frame_h
            )
            .expect("svg write");
        }

        if !scene.graticule.is_empty() {
            let mut d = String::new();
            for line in &scene.graticule {
                write_polyline_svg(&mut d, line);
            }
            if !d.is_empty() {
                write!(
                    out,
                    r#"<path d="{d}" fill="none" stroke="{border_hex}" stroke-opacity="0.45" stroke-width="0.75"/>"#,
                )
                .expect("svg write");
            }
        }

        for region in &scene.regions {
            let alpha = strength(region.value, low, high);
            let mut d = String::new();
            for ring in &region.rings {
                if ring.len() >= 3 {
                    write_polygon_svg(&mut d, ring);
                }
            }
            if !d.is_empty() {
                write!(
                    out,
                    r#"<path d="{d}" fill="{region_hex}" fill-opacity="{alpha:.2}" stroke="{bg_hex}" stroke-opacity="0.8" stroke-width="1"/>"#,
                )
                .expect("svg write");
            }
        }

        for pt in &scene.points {
            write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{point_hex}" fill-opacity="0.75" stroke="{point_hex}" stroke-width="1"/>"#,
                pt.center.0, pt.center.1, pt.radius
            )
            .expect("svg write");
        }

        out.push_str("</svg>");
        out
    }
}

impl Styled for GeoChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for GeoChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<GeoState> =
            window.use_keyed_state((self.id.clone(), "geo"), cx, |_, _| GeoState::default());
        let (bounds, yaw_offset, pitch_offset, zoom, pan, hover, show_reset) = {
            let s = state.read(cx);
            let pick_count = self.points.len() + self.regions.len();
            (
                s.bounds,
                s.yaw_offset,
                s.pitch_offset,
                s.zoom.clamp(0.5, 12.0),
                s.pan,
                s.pointed(pick_count),
                s.is_modified(),
            )
        };
        let view_rotate = (
            self.rotate.0 + yaw_offset,
            (self.rotate.1 + pitch_offset).clamp(-89.0, 89.0),
        );
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pad = f32::from(sizes.inset.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let scene = build_scene(
            &self,
            box_size,
            pad,
            GeoView {
                rotate: view_rotate,
                zoom,
                pan,
            },
        );
        let region_ink = tint(&colors, 0);
        let point_ink = if self.regions.is_empty() {
            tint(&colors, 0)
        } else {
            tint(&colors, 1)
        };
        let (low, high) = region_extent(&self.regions);
        let format = self.format.clone();

        let tooltip = hover.and_then(|ix| {
            if ix < self.points.len() {
                let pt = scene.points.iter().find(|p| p.index == ix)?;
                let raw = &self.points[ix];
                let card = ChartTooltip::new(raw.name.clone())
                    .row(Some(point_ink), "Value", format(raw.value))
                    .row(None, "Coords", format!("{:.1}°, {:.1}°", raw.lat, raw.lon));
                Some(anchored(pt.center, pt.center.0 > box_size.0 * 0.55, card))
            } else {
                let r_ix = ix - self.points.len();
                let reg = scene.regions.iter().find(|r| r.index == r_ix)?;
                let raw = &self.regions[r_ix];
                let alpha = strength(raw.value, low, high);
                let card = ChartTooltip::new(raw.name.clone()).row(
                    Some(region_ink.opacity(alpha)),
                    "Value",
                    format(raw.value),
                );
                Some(anchored(reg.anchor, reg.anchor.0 > box_size.0 * 0.55, card))
            }
        });

        let hit_points: Vec<(usize, (f32, f32), f32)> = scene
            .points
            .iter()
            .map(|p| (p.index, p.center, p.radius))
            .collect();
        type HitRegionRings = (usize, Vec<Vec<(f32, f32)>>);
        let hit_regions: Vec<HitRegionRings> = scene
            .regions
            .iter()
            .map(|r| (r.index, r.rings.clone()))
            .collect();
        let points_offset = self.points.len();

        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let globe = scene.globe;
        let frame_rect = (scene.frame_x, scene.frame_y, scene.frame_w, scene.frame_h);
        let graticule = scene.graticule;
        let paint_regions = scene.regions;
        let paint_points = scene.points;
        let min_point_hit = if touch_mode { 24.0 } else { 8.0 };

        let find: Rc<dyn Fn((f32, f32)) -> Option<usize>> = Rc::new(move |pos| {
            if let Some((ix, _, _)) = hit_points
                .iter()
                .filter_map(|&(ix, c, r)| {
                    let d2 = (pos.0 - c.0).powi(2) + (pos.1 - c.1).powi(2);
                    let limit = (r + 5.0).max(min_point_hit);
                    (d2 <= limit * limit).then_some((ix, c, d2))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
            {
                return Some(ix);
            }
            for (r_ix, rings) in hit_regions.iter().rev() {
                let mut inside = false;
                for ring in rings {
                    if point_in_polygon(pos, ring) {
                        inside = !inside;
                    }
                }
                if inside {
                    return Some(points_offset + *r_ix);
                }
            }
            None
        });

        let base_rotate = self.rotate;
        let (down_state, move_state, up_state, scroll_state, pinch_state, hover_state) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_find, move_find) = (find.clone(), find);
        let clear_drag = move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
            up_state.update(cx, |s, _| {
                s.drag_last = None;
            });
        };

        let reset_button = show_reset.then(|| {
            let reset_state = state.clone();
            div()
                .absolute()
                .top_1()
                .right_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    Button::new((self.id.clone(), "geo-reset"), "Reset")
                        .debug_selector(|| "geo-reset".into())
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, _, cx| {
                            reset_state.update(cx, |s, cx| {
                                s.reset();
                                cx.notify();
                            });
                        }),
                )
        });

        let map_area = div()
            .debug_selector(|| "geo-plot".into())
            .id((self.id.clone(), "geo-map"))
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, down_state.read(cx).bounds.origin);
                down_state.update(cx, |s, cx| {
                    if event.click_count >= 2 {
                        s.reset();
                        s.hover = down_find(pos);
                        cx.notify();
                    } else {
                        s.drag_last = Some(pos);
                        let next = down_find(pos);
                        if s.hover != next {
                            s.hover = next;
                            cx.notify();
                        }
                    }
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, move_state.read(cx).bounds.origin);
                move_state.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(prev) = s.drag_last
                    {
                        let dx = pos.0 - prev.0;
                        let dy = pos.1 - prev.1;
                        s.drag_last = Some(pos);
                        if dx.abs() > 0.0 || dy.abs() > 0.0 {
                            s.rotate_by(dx, dy, base_rotate.1);
                            s.hover = None;
                            cx.notify();
                        }
                        return;
                    }
                    s.drag_last = None;
                    let next = move_find(pos);
                    if s.hover != next {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, clear_drag.clone())
            .on_mouse_up_out(MouseButton::Left, clear_drag)
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scroll_state.update(cx, |s, cx| {
                            s.rotate_by(dx, dy, base_rotate.1);
                            s.hover = None;
                            cx.notify();
                        });
                    }
                } else if dy.abs() >= 0.01 {
                    let factor = (dy * 0.0025).exp().clamp(0.5, 2.0);
                    scroll_state.update(cx, |s, cx| {
                        if s.zoom_by(factor) {
                            cx.notify();
                        }
                    });
                }
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                pinch_state.update(cx, |s, cx| {
                    s.drag_last = None;
                    if ev.phase != TouchPhase::Cancelled
                        && ev.delta.is_finite()
                        && ev.delta.abs() > 0.0001
                    {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if s.zoom_by(scale) {
                            cx.notify();
                        }
                    }
                });
                cx.stop_propagation();
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    hover_state.update(cx, |s, cx| {
                        if s.hover.take().is_some() {
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
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            if let Some((cx_px, cy_px, r_px)) = globe
                                && r_px > 1.0
                            {
                                let segs = 72;
                                let mut fill_circle = PathBuilder::fill();
                                let mut edge_circle = PathBuilder::stroke(hairline);
                                let p0 = (cx_px + r_px, cy_px);
                                fill_circle.move_to(at(origin, p0));
                                edge_circle.move_to(at(origin, p0));
                                for s in 1..=segs {
                                    let a = std::f32::consts::TAU * s as f32 / segs as f32;
                                    let pt = (cx_px + r_px * a.cos(), cy_px + r_px * a.sin());
                                    fill_circle.line_to(at(origin, pt));
                                    edge_circle.line_to(at(origin, pt));
                                }
                                fill_circle.close();
                                edge_circle.close();
                                finish(fill_circle, colors.fg.opacity(0.03), window);
                                finish(edge_circle, colors.border.opacity(0.7), window);
                            } else if frame_rect.2 > 1.0 && frame_rect.3 > 1.0 {
                                let (fx, fy, fw, fh) = frame_rect;
                                let mut box_path = PathBuilder::stroke(hairline);
                                box_path.move_to(at(origin, (fx, fy)));
                                box_path.line_to(at(origin, (fx + fw, fy)));
                                box_path.line_to(at(origin, (fx + fw, fy + fh)));
                                box_path.line_to(at(origin, (fx, fy + fh)));
                                box_path.close();
                                finish(box_path, colors.border.opacity(0.55), window);
                            }

                            for line in &graticule {
                                if line.len() < 2 {
                                    continue;
                                }
                                let mut path = PathBuilder::stroke(hairline);
                                path.move_to(at(origin, line[0]));
                                for &pt in &line[1..] {
                                    path.line_to(at(origin, pt));
                                }
                                finish(path, colors.border.opacity(0.45), window);
                            }

                            for reg in &paint_regions {
                                let is_hovered = hover == Some(points_offset + reg.index);
                                let alpha = strength(reg.value, low, high);
                                let fill_color = region_ink.opacity(if is_hovered {
                                    (alpha + 0.14).min(1.0)
                                } else {
                                    alpha
                                });
                                let border_color = if is_hovered {
                                    colors.fg
                                } else {
                                    colors.bg.opacity(0.8)
                                };
                                let border_w = if is_hovered { stroke } else { hairline };
                                for ring_pts in &reg.rings {
                                    if ring_pts.len() < 3 {
                                        continue;
                                    }
                                    let mut fill_path = PathBuilder::fill();
                                    let mut edge_path = PathBuilder::stroke(border_w);
                                    fill_path.move_to(at(origin, ring_pts[0]));
                                    edge_path.move_to(at(origin, ring_pts[0]));
                                    for &pt in &ring_pts[1..] {
                                        fill_path.line_to(at(origin, pt));
                                        edge_path.line_to(at(origin, pt));
                                    }
                                    fill_path.close();
                                    edge_path.close();
                                    finish(fill_path, fill_color, window);
                                    finish(edge_path, border_color, window);
                                }
                            }

                            for pt in &paint_points {
                                let is_hovered = hover == Some(pt.index);
                                let edge = if is_hovered { colors.fg } else { point_ink };
                                let width = if is_hovered { stroke } else { hairline };
                                ring(
                                    at(origin, pt.center),
                                    Pixels::from(pt.radius),
                                    (point_ink.opacity(0.75), edge),
                                    width,
                                    window,
                                );
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(tooltip)
            .children(reset_button)
            .child(measure(state, |s| &mut s.bounds));

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        if self.regions.is_empty() {
            root.flex().flex_col().child(map_area)
        } else {
            let legend = key(steps(region_ink), ("Low", "High"), cx);
            root.flex().flex_col().gap_2().child(map_area).child(legend)
        }
    }
}

struct GeoState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    yaw_offset: f64,
    pitch_offset: f64,
    zoom: f32,
    pan: (f32, f32),
    drag_last: Option<(f32, f32)>,
}

impl Default for GeoState {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            hover: None,
            yaw_offset: 0.0,
            pitch_offset: 0.0,
            zoom: 1.0,
            pan: (0.0, 0.0),
            drag_last: None,
        }
    }
}

impl GeoState {
    fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }

    fn is_modified(&self) -> bool {
        self.zoom != 1.0
            || self.yaw_offset != 0.0
            || self.pitch_offset != 0.0
            || self.pan != (0.0, 0.0)
    }

    fn rotate_by(&mut self, dx: f32, dy: f32, base_pitch: f64) {
        let deg_per_px = 0.45 / self.zoom.max(0.5) as f64;
        self.yaw_offset += dx as f64 * deg_per_px;
        let next_pitch =
            (base_pitch + self.pitch_offset - dy as f64 * deg_per_px).clamp(-89.0, 89.0);
        self.pitch_offset = next_pitch - base_pitch;
    }

    fn zoom_by(&mut self, factor: f32) -> bool {
        let next = (self.zoom * factor).clamp(0.5, 12.0);
        if (next - self.zoom).abs() > f32::EPSILON {
            self.zoom = next;
            true
        } else {
            false
        }
    }

    fn reset(&mut self) {
        self.yaw_offset = 0.0;
        self.pitch_offset = 0.0;
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.drag_last = None;
    }
}

#[derive(Clone, Copy)]
struct GeoView {
    rotate: (f64, f64),
    zoom: f32,
    pan: (f32, f32),
}

fn local_pos(position: Point<Pixels>, origin: Point<Pixels>) -> (f32, f32) {
    let offset = position - origin;
    (f32::from(offset.x), f32::from(offset.y))
}

#[derive(Clone, Debug)]
struct ProjectedRegion {
    index: usize,
    value: f64,
    rings: Vec<Vec<(f32, f32)>>,
    anchor: (f32, f32),
}

#[derive(Clone, Debug)]
struct ProjectedPoint {
    index: usize,
    center: (f32, f32),
    radius: f32,
}

struct GeoScene {
    globe: Option<(f32, f32, f32)>,
    frame_x: f32,
    frame_y: f32,
    frame_w: f32,
    frame_h: f32,
    graticule: Vec<Vec<(f32, f32)>>,
    regions: Vec<ProjectedRegion>,
    points: Vec<ProjectedPoint>,
}

fn validate_ring(ring: impl IntoIterator<Item = (f64, f64)>) -> Vec<(f64, f64)> {
    let pts: Vec<(f64, f64)> = ring.into_iter().collect();
    for &(lon, lat) in &pts {
        assert!(
            lon.is_finite()
                && (-180.0..=180.0).contains(&lon)
                && lat.is_finite()
                && (-90.0..=90.0).contains(&lat),
            "geo ring coordinates must be within lon -180..=180 and lat -90..=90"
        );
    }
    pts
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn svg_hex(color: Hsla) -> String {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b)
    )
}

fn write_polyline_svg(out: &mut String, pts: &[(f32, f32)]) {
    if let Some((first, rest)) = pts.split_first() {
        write!(out, "M{:.1},{:.1}", first.0, first.1).expect("svg write");
        for &(x, y) in rest {
            write!(out, " L{x:.1},{y:.1}").expect("svg write");
        }
    }
}

fn write_polygon_svg(out: &mut String, pts: &[(f32, f32)]) {
    write_polyline_svg(out, pts);
    out.push_str(" Z ");
}

fn region_extent(regions: &[GeoRegion]) -> (f64, f64) {
    if regions.is_empty() {
        return (0.0, 1.0);
    }
    regions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), r| {
            (lo.min(r.value), hi.max(r.value))
        })
}

fn strength(value: f64, low: f64, high: f64) -> f32 {
    if high > low {
        0.14 + 0.86 * ((value - low) / (high - low)) as f32
    } else {
        0.55
    }
}

fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.14, 0.35, 0.56, 0.78, 1.0].map(|s| ink.opacity(s))
}

fn centroid_deg(ring: &[(f64, f64)]) -> Option<(f64, f64)> {
    if ring.is_empty() {
        return None;
    }
    let (sx, sy) = ring
        .iter()
        .fold((0.0, 0.0), |(ax, ay), &(x, y)| (ax + x, ay + y));
    let n = ring.len() as f64;
    Some((sx / n, sy / n))
}

/// Rotate `(lon_deg, lat_deg)` by `(yaw_deg, pitch_deg)` into a 3D unit vector `(x, y, z)` (`+x` viewer, `+y` East, `+z` North).
fn rotate_vec(lon_deg: f64, lat_deg: f64, (yaw_deg, pitch_deg): (f64, f64)) -> [f64; 3] {
    let dl = (lon_deg + yaw_deg).to_radians();
    let phi = lat_deg.to_radians();
    let x = phi.cos() * dl.cos();
    let y = phi.cos() * dl.sin();
    let z = phi.sin();
    let p = pitch_deg.to_radians();
    let (sp, cp) = p.sin_cos();
    let x1 = x * cp - z * sp;
    let z1 = x * sp + z * cp;
    [x1, y, z1]
}

const MERCATOR_MAX_LAT_RAD: f64 = 85.0 * PI / 180.0;
const STEREO_CLIP_COS: f64 = -0.5;
const STEREO_CLIP_SCALE: f64 = 1.732_050_807_568_877_2;

fn mercator_max_y() -> f64 {
    (FRAC_PI_4 + MERCATOR_MAX_LAT_RAD * 0.5).tan().ln()
}

fn projection_bounds(proj: GeoProjection) -> (f64, f64) {
    match proj {
        GeoProjection::Equirectangular => (PI, FRAC_PI_2),
        GeoProjection::Mercator => (PI, mercator_max_y()),
        GeoProjection::Orthographic | GeoProjection::Stereographic => (1.0, 1.0),
    }
}

fn clip_threshold(proj: GeoProjection) -> Option<f64> {
    match proj {
        GeoProjection::Orthographic => Some(0.0),
        GeoProjection::Stereographic => Some(STEREO_CLIP_COS),
        GeoProjection::Equirectangular | GeoProjection::Mercator => None,
    }
}

/// Project a rotated 3D unit vector `[x, y, z]` to normalized `(nx, ny)` coordinates (`+ny` North).
fn project_vec(v: [f64; 3], proj: GeoProjection) -> Option<(f64, f64)> {
    let [x, y, z] = v;
    match proj {
        GeoProjection::Equirectangular => {
            let lon = y.atan2(x);
            let lat = z.clamp(-1.0, 1.0).asin();
            Some((lon, lat))
        }
        GeoProjection::Mercator => {
            let lon = y.atan2(x);
            let lat = z
                .clamp(-1.0, 1.0)
                .asin()
                .clamp(-MERCATOR_MAX_LAT_RAD, MERCATOR_MAX_LAT_RAD);
            let ny = (FRAC_PI_4 + lat * 0.5).tan().ln();
            Some((lon, ny))
        }
        GeoProjection::Orthographic => {
            if x < 0.0 {
                None
            } else {
                Some((y, z))
            }
        }
        GeoProjection::Stereographic => {
            if x < STEREO_CLIP_COS {
                None
            } else {
                let denom = (1.0 + x) * STEREO_CLIP_SCALE;
                if denom <= 1e-6 {
                    None
                } else {
                    Some((y / denom, z / denom))
                }
            }
        }
    }
}

/// Subdivide spherical edges larger than ~5 deg so projected arcs and horizon clips remain smooth.
fn subdivide_ring(vecs: &[[f64; 3]]) -> Vec<[f64; 3]> {
    if vecs.len() < 2 {
        return vecs.to_vec();
    }
    let max_angle = 5.0_f64.to_radians();
    let mut out = Vec::with_capacity(vecs.len() * 2);
    for (ix, &a) in vecs.iter().enumerate() {
        let b = vecs[(ix + 1) % vecs.len()];
        out.push(a);
        let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        let angle = dot.acos();
        if angle > max_angle {
            let steps = (angle / max_angle).ceil() as usize;
            for s in 1..steps {
                let t = s as f64 / steps as f64;
                let p = [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ];
                let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
                out.push([p[0] / len, p[1] / len, p[2] / len]);
            }
        }
    }
    out
}

fn horizon_intersect(a: [f64; 3], b: [f64; 3], threshold: f64) -> [f64; 3] {
    let denom = b[0] - a[0];
    let t = if denom.abs() < 1e-12 {
        0.5
    } else {
        ((threshold - a[0]) / denom).clamp(0.0, 1.0)
    };
    let y = a[1] + (b[1] - a[1]) * t;
    let z = a[2] + (b[2] - a[2]) * t;
    let r_yz = (1.0 - threshold * threshold).max(0.0).sqrt();
    let len = (y * y + z * z).sqrt();
    if len > 1e-12 {
        [threshold, y / len * r_yz, z / len * r_yz]
    } else {
        [threshold, r_yz, 0.0]
    }
}

/// Sutherland-Hodgman clip of a closed 3D spherical ring against `v[0] >= threshold`.
fn clip_ring_horizon(vecs: &[[f64; 3]], threshold: f64) -> Vec<[f64; 3]> {
    if vecs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(vecs.len());
    let n = vecs.len();
    for (ix, &curr) in vecs.iter().enumerate() {
        let prev = vecs[(ix + n - 1) % n];
        let prev_in = prev[0] >= threshold;
        let curr_in = curr[0] >= threshold;
        match (prev_in, curr_in) {
            (true, true) => out.push(curr),
            (false, true) => {
                out.push(horizon_intersect(prev, curr, threshold));
                out.push(curr);
            }
            (true, false) => {
                out.push(horizon_intersect(prev, curr, threshold));
            }
            (false, false) => {}
        }
    }
    out
}

fn project_polyline(
    pts_deg: &[(f64, f64)],
    rotate: (f64, f64),
    proj: GeoProjection,
    to_px: impl Fn((f64, f64)) -> (f32, f32),
) -> Vec<Vec<(f32, f32)>> {
    let vecs: Vec<[f64; 3]> = pts_deg
        .iter()
        .map(|&(lon, lat)| rotate_vec(lon, lat, rotate))
        .collect();
    let mut segments: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();

    if let Some(threshold) = clip_threshold(proj) {
        for win in vecs.windows(2) {
            let (a, b) = (win[0], win[1]);
            let (a_in, b_in) = (a[0] >= threshold, b[0] >= threshold);
            match (a_in, b_in) {
                (true, true) => {
                    if current.is_empty()
                        && let Some(p) = project_vec(a, proj)
                    {
                        current.push(to_px(p));
                    }
                    if let Some(p) = project_vec(b, proj) {
                        current.push(to_px(p));
                    }
                }
                (true, false) => {
                    if current.is_empty()
                        && let Some(p) = project_vec(a, proj)
                    {
                        current.push(to_px(p));
                    }
                    let mid = horizon_intersect(a, b, threshold);
                    if let Some(p) = project_vec(mid, proj) {
                        current.push(to_px(p));
                    }
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
                (false, true) => {
                    let mid = horizon_intersect(a, b, threshold);
                    if let Some(p) = project_vec(mid, proj) {
                        current.push(to_px(p));
                    }
                    if let Some(p) = project_vec(b, proj) {
                        current.push(to_px(p));
                    }
                }
                (false, false) => {
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
            }
        }
    } else {
        let mut prev_nx: Option<f64> = None;
        for &v in &vecs {
            if let Some((nx, ny)) = project_vec(v, proj) {
                if let Some(px_prev) = prev_nx
                    && (nx - px_prev).abs() > PI
                {
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
                current.push(to_px((nx, ny)));
                prev_nx = Some(nx);
            }
        }
    }

    if current.len() >= 2 {
        segments.push(current);
    }
    segments
}

fn build_scene(chart: &GeoChart, size: (f32, f32), pad: f32, view: GeoView) -> GeoScene {
    let avail_w = (size.0 - pad * 2.0).max(0.0);
    let avail_h = (size.1 - pad * 2.0).max(0.0);
    if avail_w <= 1.0 || avail_h <= 1.0 {
        return GeoScene {
            globe: None,
            frame_x: 0.0,
            frame_y: 0.0,
            frame_w: 0.0,
            frame_h: 0.0,
            graticule: Vec::new(),
            regions: Vec::new(),
            points: Vec::new(),
        };
    }

    let (dom_w, dom_h) = projection_bounds(chart.projection);
    let fit_scale = (avail_w / (2.0 * dom_w as f32)).min(avail_h / (2.0 * dom_h as f32));
    let scale = fit_scale * view.zoom.clamp(0.5, 12.0);
    let (cx_px, cy_px) = (size.0 * 0.5 + view.pan.0, size.1 * 0.5 + view.pan.1);
    let to_px = |(nx, ny): (f64, f64)| (cx_px + nx as f32 * scale, cy_px - ny as f32 * scale);

    let globe = match chart.projection {
        GeoProjection::Orthographic | GeoProjection::Stereographic => Some((cx_px, cy_px, scale)),
        GeoProjection::Equirectangular | GeoProjection::Mercator => None,
    };
    let frame_w = 2.0 * dom_w as f32 * scale;
    let frame_h = 2.0 * dom_h as f32 * scale;
    let frame_x = cx_px - frame_w * 0.5;
    let frame_y = cy_px - frame_h * 0.5;

    let mut graticule = Vec::new();
    if chart.show_graticule {
        let step = chart.graticule_step;
        let lat_limit = match chart.projection {
            GeoProjection::Mercator => 85.0,
            _ => 90.0,
        };
        let mut lon = -180.0_f64;
        while lon <= 180.0 + 1e-6 {
            let mut meridian = Vec::new();
            let mut lat = -lat_limit;
            while lat < lat_limit {
                meridian.push((lon.clamp(-180.0, 180.0), lat));
                lat += 4.0;
            }
            meridian.push((lon.clamp(-180.0, 180.0), lat_limit));
            graticule.extend(project_polyline(
                &meridian,
                view.rotate,
                chart.projection,
                to_px,
            ));
            lon += step;
        }
        let mut lat = -((90.0 / step).floor() * step);
        while lat <= 90.0 - 1e-6 {
            if lat.abs() <= lat_limit {
                let mut parallel = Vec::new();
                let mut l = -180.0;
                while l < 180.0 {
                    parallel.push((l, lat));
                    l += 4.0;
                }
                parallel.push((180.0, lat));
                graticule.extend(project_polyline(
                    &parallel,
                    view.rotate,
                    chart.projection,
                    to_px,
                ));
            }
            lat += step;
        }
    }

    let threshold = clip_threshold(chart.projection);
    let mut regions = Vec::new();
    for (r_ix, region) in chart.regions.iter().enumerate() {
        let mut projected_rings = Vec::new();
        for ring in &region.rings {
            if ring.len() < 3 {
                continue;
            }
            let vecs: Vec<[f64; 3]> = ring
                .iter()
                .map(|&(lon, lat)| rotate_vec(lon, lat, view.rotate))
                .collect();
            let vecs = if let Some(thresh) = threshold {
                let dense = subdivide_ring(&vecs);
                clip_ring_horizon(&dense, thresh)
            } else {
                vecs
            };
            let px_ring: Vec<(f32, f32)> = vecs
                .into_iter()
                .filter_map(|v| project_vec(v, chart.projection))
                .map(to_px)
                .collect();
            if px_ring.len() >= 3 {
                projected_rings.push(px_ring);
            }
        }
        if !projected_rings.is_empty() {
            let anchor = polygon_anchor(&projected_rings[0]);
            regions.push(ProjectedRegion {
                index: r_ix,
                value: region.value,
                rings: projected_rings,
                anchor,
            });
        }
    }

    let max_pt_val = chart
        .points
        .iter()
        .map(|p| p.value.max(0.0))
        .fold(0.0_f64, f64::max);
    let mut points = Vec::new();
    for (p_ix, pt) in chart.points.iter().enumerate() {
        let v = rotate_vec(pt.lon, pt.lat, view.rotate);
        if let Some(norm) = project_vec(v, chart.projection) {
            let center = to_px(norm);
            let radius = if max_pt_val > 0.0 {
                let ratio = (pt.value.max(0.0) / max_pt_val).sqrt() as f32;
                3.5 + 8.5 * ratio
            } else {
                5.5
            };
            points.push(ProjectedPoint {
                index: p_ix,
                center,
                radius,
            });
        }
    }

    GeoScene {
        globe,
        frame_x,
        frame_y,
        frame_w,
        frame_h,
        graticule,
        regions,
        points,
    }
}

fn polygon_anchor(ring: &[(f32, f32)]) -> (f32, f32) {
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for &(x, y) in ring {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
}

fn point_in_polygon((px, py): (f32, f32), ring: &[(f32, f32)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for (i, &(xi, yi)) in ring.iter().enumerate() {
        let (xj, yj) = ring[j];
        let intersects = ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_and_projections_clip_and_clamp_accurately() {
        let v = rotate_vec(127.0, 37.5, (-127.0, -37.5));
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!(v[1].abs() < 1e-6);
        assert!(v[2].abs() < 1e-6);

        let front = rotate_vec(10.0, 0.0, (0.0, 0.0));
        let back = rotate_vec(170.0, 0.0, (0.0, 0.0));
        assert!(project_vec(front, GeoProjection::Orthographic).is_some());
        assert!(project_vec(back, GeoProjection::Orthographic).is_none());

        let pole = rotate_vec(0.0, 90.0, (0.0, 0.0));
        let (mx, my) = project_vec(pole, GeoProjection::Mercator).expect("clamped mercator");
        assert!(mx.abs() < 1e-6);
        assert!((my - mercator_max_y()).abs() < 1e-6);

        let center = project_vec(
            rotate_vec(0.0, 0.0, (0.0, 0.0)),
            GeoProjection::Stereographic,
        );
        assert_eq!(center, Some((0.0, 0.0)));
        let edge = project_vec(
            rotate_vec(90.0, 0.0, (0.0, 0.0)),
            GeoProjection::Stereographic,
        )
        .expect("90 deg visible in stereographic");
        assert!(edge.0.hypot(edge.1) < 1.0);
    }

    #[test]
    fn horizon_clipping_and_point_in_polygon_handle_polygons() {
        let poly = [(-20.0, -20.0), (20.0, -20.0), (20.0, 20.0), (-20.0, 20.0)];
        let screen = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        assert!(point_in_polygon((50.0, 50.0), &screen));
        assert!(!point_in_polygon((150.0, 50.0), &screen));

        let vecs: Vec<[f64; 3]> = [(-30.0, -10.0), (130.0, -10.0), (130.0, 10.0), (-30.0, 10.0)]
            .into_iter()
            .map(|(lon, lat)| rotate_vec(lon, lat, (0.0, 0.0)))
            .collect();
        let clipped = clip_ring_horizon(&subdivide_ring(&vecs), 0.0);
        assert!(clipped.len() >= 3);
        assert!(clipped.iter().all(|v| v[0] >= -1e-9));
        let _ = GeoRegion::new("Box", 10.0, poly);
    }

    #[test]
    fn geo_state_clamps_rotation_and_zoom_and_scales_scene() {
        let mut state = GeoState::default();
        assert!(!state.is_modified());

        state.rotate_by(100.0, -500.0, 0.0);
        assert!(state.is_modified());
        assert!(state.yaw_offset > 0.0);
        assert_eq!(state.pitch_offset, 89.0);

        assert!(state.zoom_by(20.0));
        assert_eq!(state.zoom, 12.0);
        assert!(!state.zoom_by(2.0));
        assert!(state.zoom_by(0.01));
        assert_eq!(state.zoom, 0.5);

        let chart = GeoChart::new("scale-check")
            .projection(GeoProjection::Orthographic)
            .point(GeoPoint::new("Center", 0.0, 0.0, 10.0));
        let base_scene = build_scene(
            &chart,
            (300.0, 200.0),
            10.0,
            GeoView {
                rotate: (0.0, 0.0),
                zoom: 1.0,
                pan: (0.0, 0.0),
            },
        );
        let zoomed_scene = build_scene(
            &chart,
            (300.0, 200.0),
            10.0,
            GeoView {
                rotate: (0.0, 0.0),
                zoom: 2.0,
                pan: (0.0, 0.0),
            },
        );
        assert!(
            (zoomed_scene.globe.expect("globe").2 - base_scene.globe.expect("globe").2 * 2.0).abs()
                < 1e-4
        );

        state.reset();
        assert!(!state.is_modified());
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.yaw_offset, 0.0);
        assert_eq!(state.pitch_offset, 0.0);
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn geo_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, px};

        struct GeoHarness;
        impl Render for GeoHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).h(px(260.0)).child(
                    GeoChart::new("geo-test")
                        .projection(GeoProjection::Orthographic)
                        .rotate(-10.0, -10.0)
                        .graticule(30.0)
                        .region(
                            GeoRegion::new(
                                "Atlantis",
                                42.0,
                                [(-20.0, -20.0), (40.0, -20.0), (40.0, 40.0), (-20.0, 40.0)],
                            )
                            .ring([
                                (5.0, 5.0),
                                (15.0, 5.0),
                                (15.0, 15.0),
                                (5.0, 15.0),
                            ]),
                        )
                        .point(GeoPoint::new("Capital", 10.0, 10.0, 100.0)),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            let chart = GeoChart::new("export")
                .projection(GeoProjection::Mercator)
                .region(GeoRegion::new(
                    "Alpha, Zone",
                    15.0,
                    [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)],
                ))
                .point(GeoPoint::new("Hub", 0.0, 0.0, 50.0));
            let svg = chart.svg(360.0, 240.0, cx);
            assert!(svg.starts_with("<svg") && svg.contains("<path") && svg.contains("<circle"));
            let csv = chart.csv();
            assert!(csv.starts_with("kind,name,lon,lat,value\n"));
            assert!(csv.contains("\"Alpha, Zone\""));
            assert!(csv.contains("point,Hub,0,0,50"));
        });

        let (_, vc) = cx.add_window_view(|_, _| GeoHarness);
        settle(vc);
        assert!(vc.debug_bounds("chart-root").is_some());
        let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
        vc.simulate_mouse_move(plot.center(), None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn geo_chart_supports_drag_rotation_zoom_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

        struct GeoManipHarness;
        impl Render for GeoManipHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).h(px(260.0)).child(
                    GeoChart::new("geo-manip")
                        .projection(GeoProjection::Orthographic)
                        .point(GeoPoint::new("Origin", 0.0, 0.0, 50.0)),
                )
            }
        }

        cx.update(Theme::init);
        let (_, vc) = cx.add_window_view(|_, _| GeoManipHarness);
        settle(vc);

        let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
        let center = plot.center();
        assert!(vc.debug_bounds("geo-reset").is_none());

        // Hover at center opens tooltip on the origin point.
        vc.simulate_mouse_move(center, None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        // Left-button drag rotates the globe away from the origin point and reveals Reset.
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(
            center + point(px(140.0), px(-30.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            center + point(px(140.0), px(-30.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_some());
        assert!(vc.debug_bounds("chart-tooltip").is_none());

        // Double-click resets rotation and hides Reset; origin is back at center.
        for click_count in [1, 2] {
            vc.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position: center,
                modifiers: Modifiers::none(),
                click_count,
                first_mouse: false,
            });
            vc.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position: center,
                modifiers: Modifiers::none(),
                click_count,
            });
            settle(vc);
        }
        assert!(vc.debug_bounds("geo-reset").is_none());
        vc.simulate_mouse_move(center, None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        // Pointer wheel scroll zooms in and shows Reset.
        vc.simulate_event(ScrollEvent {
            position: center,
            delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(vc);
        let reset_btn = vc
            .debug_bounds("geo-reset")
            .expect("reset after scroll zoom");

        // Clicking Reset restores initial state.
        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_none());

        // Pinch zoom and touch two-axis scroll also update state and double-click resets.
        vc.simulate_event(PinchEvent {
            position: center,
            delta: 0.4,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_some());

        vc.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: center,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: center,
            modifiers: Modifiers::none(),
            click_count: 2,
        });
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_none());

        vc.simulate_event(ScrollEvent {
            position: center,
            delta: ScrollDelta::Pixels(point(px(40.0), px(-20.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_some());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn geo_chart_touch_mode_uses_24px_hit_radius_clears_drag_on_pinch_and_flips_tooltip(
        cx: &mut gpui::TestAppContext,
    ) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, point, px, size};

        struct GeoTouchHarness;
        impl Render for GeoTouchHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(260.0)).h(px(220.0)).child(
                    GeoChart::new("geo-touch")
                        .projection(GeoProjection::Equirectangular)
                        .point(GeoPoint::new("Center", 0.0, 0.0, 40.0))
                        .point(GeoPoint::new("East", 60.0, 0.0, 80.0)),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });
        let (_, vc) = cx.add_window_view(|_, _| GeoTouchHarness);
        vc.simulate_resize(size(px(260.0), px(220.0)));
        settle(vc);

        let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
        let center = plot.center();

        // Tapping 18px left of the center point hits it via the >=24px touch radius.
        let near_center = center - point(px(18.0), px(0.0));
        vc.simulate_mouse_down(near_center, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(near_center, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        // Tapping the east point (> 0.55 * width) flips its tooltip to the left.
        let east = point(
            plot.left() + plot.size.width * (0.5 + 60.0 / 360.0),
            center.y,
        );
        vc.simulate_mouse_down(east, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(east, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip = vc.debug_bounds("chart-tooltip").expect("east tooltip");
        assert!(tip.center().x < east.x);

        // Pinching after mouse down clears drag_last so subsequent move does not rotate.
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        vc.simulate_event(PinchEvent {
            position: center,
            delta: 0.35,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        vc.simulate_mouse_move(
            center + point(px(80.0), px(0.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            center + point(px(80.0), px(0.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);

        let reset_btn = vc.debug_bounds("geo-reset").expect("geo reset after pinch");
        // Center point stayed unrotated because pinch cleared drag_last.
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("geo-reset").is_none());
    }
}
