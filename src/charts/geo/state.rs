use super::*;

pub(super) struct GeoState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) yaw_offset: f64,
    pub(super) pitch_offset: f64,
    pub(super) zoom: f32,
    pub(super) pan: (f32, f32),
    pub(super) drag_last: Option<(f32, f32)>,
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
    pub(super) fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }

    pub(super) fn is_modified(&self) -> bool {
        self.zoom != 1.0
            || self.yaw_offset != 0.0
            || self.pitch_offset != 0.0
            || self.pan != (0.0, 0.0)
    }

    pub(super) fn rotate_by(&mut self, dx: f32, dy: f32, base_pitch: f64) {
        let deg_per_px = 0.45 / self.zoom.max(0.5) as f64;
        self.yaw_offset += dx as f64 * deg_per_px;
        let next_pitch =
            (base_pitch + self.pitch_offset - dy as f64 * deg_per_px).clamp(-89.0, 89.0);
        self.pitch_offset = next_pitch - base_pitch;
    }

    pub(super) fn zoom_by(&mut self, factor: f32) -> bool {
        let next = (self.zoom * factor).clamp(0.5, 12.0);
        if (next - self.zoom).abs() > f32::EPSILON {
            self.zoom = next;
            true
        } else {
            false
        }
    }

    pub(super) fn reset(&mut self) {
        self.yaw_offset = 0.0;
        self.pitch_offset = 0.0;
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.drag_last = None;
    }
}

#[derive(Clone, Copy)]
pub(super) struct GeoView {
    pub(super) rotate: (f64, f64),
    pub(super) zoom: f32,
    pub(super) pan: (f32, f32),
}

pub(super) fn local_pos(position: Point<Pixels>, origin: Point<Pixels>) -> (f32, f32) {
    let offset = position - origin;
    (f32::from(offset.x), f32::from(offset.y))
}

#[derive(Clone, Debug)]
pub(super) struct ProjectedRegion {
    pub(super) index: usize,
    pub(super) value: f64,
    pub(super) rings: Vec<Vec<(f32, f32)>>,
    pub(super) anchor: (f32, f32),
}

#[derive(Clone, Debug)]
pub(super) struct ProjectedPoint {
    pub(super) index: usize,
    pub(super) center: (f32, f32),
    pub(super) radius: f32,
}

pub(super) struct GeoScene {
    pub(super) globe: Option<(f32, f32, f32)>,
    pub(super) frame_x: f32,
    pub(super) frame_y: f32,
    pub(super) frame_w: f32,
    pub(super) frame_h: f32,
    pub(super) graticule: Vec<Vec<(f32, f32)>>,
    pub(super) regions: Vec<ProjectedRegion>,
    pub(super) points: Vec<ProjectedPoint>,
}
