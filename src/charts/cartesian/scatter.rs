use super::*;

/// Points placed by two values; with sizes, bubbles whose areas compare. Hover a point to read it.
#[derive(IntoElement)]
pub struct ScatterChart {
    base: Div,
    id: ElementId,
    clouds: Vec<Points>,
    axes: (SharedString, SharedString),
    x_scale: ScaleMode,
    y_scale: ScaleMode,
    voronoi: bool,
    format: Format,
}

impl ScatterChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            clouds: Vec::new(),
            axes: ("x".into(), "y".into()),
            x_scale: ScaleMode::Linear,
            y_scale: ScaleMode::Linear,
            voronoi: false,
            format: Rc::new(compact),
        }
    }

    pub fn points(mut self, points: Points) -> Self {
        self.clouds.push(points);
        self
    }

    /// What the two values measure, as the tooltip names them.
    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Use a logarithmic X scale.
    pub fn x_log(mut self, base: f64) -> Self {
        self.x_scale = ScaleMode::Log(base);
        self
    }

    /// Use a logarithmic Y scale.
    pub fn y_log(mut self, base: f64) -> Self {
        self.y_scale = ScaleMode::Log(base);
        self
    }

    /// Use a symmetric logarithmic X scale.
    pub fn x_symlog(mut self, constant: f64) -> Self {
        self.x_scale = ScaleMode::Symlog(constant);
        self
    }

    /// Use a symmetric logarithmic Y scale.
    pub fn y_symlog(mut self, constant: f64) -> Self {
        self.y_scale = ScaleMode::Symlog(constant);
        self
    }

    /// Draw clipped Voronoi cells and select the nearest point anywhere in the plot.
    pub fn voronoi(mut self) -> Self {
        self.voronoi = true;
        self
    }

    /// The points as comma-separated text: a row per point with its series, x, y and size.
    pub fn csv(&self) -> String {
        points_csv(&self.clouds)
    }

    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let sheet = Sheet::new((width, height), false, Curve::Linear, cx);
        let (values, hidden) = (Vec::new(), HashSet::new());
        let scene = Scene {
            frame: sheet.frame,
            values: &values,
            hidden: &hidden,
            span: (0, 0),
            xy_view: None,
        };
        svg(
            &(self.layout(radii(cx.theme().base_rem(), cx)))(&scene),
            &sheet,
            &*self.format,
        )
    }

    fn layout(&self, radii: (f32, f32)) -> Layout {
        let (clouds, format, scales, voronoi) = (
            Rc::new(self.clouds.clone()),
            self.format.clone(),
            (self.x_scale, self.y_scale),
            self.voronoi,
        );
        Rc::new(move |scene: &Scene| {
            scatter(
                &clouds,
                scene.hidden,
                scene.frame,
                radii,
                scales,
                scene.xy_view,
                voronoi,
                &*format,
            )
        })
    }

    fn tips(&self) -> Tips {
        let (clouds, format, (x_name, y_name)) = (
            Rc::new(self.clouds.clone()),
            self.format.clone(),
            self.axes.clone(),
        );
        Rc::new(move |key, _| {
            let (cloud, ix) = locate(&clouds, key);
            let ((x, y), points) = (clouds[cloud].points[ix], &clouds[cloud]);
            let mut rows = vec![
                (None, x_name.clone(), format(x)),
                (None, y_name.clone(), format(y)),
            ];
            rows.extend(
                points
                    .sizes
                    .as_ref()
                    .map(|sizes| (None, "Size".into(), format(sizes[ix]))),
            );
            (points.name.clone(), rows)
        })
    }
}

/// The widest bubble's radius and a plain dot's, at a rem size.
fn radii(rem: gpui::Pixels, cx: &App) -> (f32, f32) {
    let theme = cx.theme();
    (
        f32::from(theme.chart().bubble.to_pixels(rem)),
        f32::from(theme.status_dot().to_pixels(rem)) / 2.0,
    )
}

/// Which cloud and which of its points a flat index names.
pub(super) fn locate(clouds: &[Points], mut key: usize) -> (usize, usize) {
    for (cloud, points) in clouds.iter().enumerate() {
        if key < points.points.len() {
            return (cloud, key);
        }
        key -= points.points.len();
    }
    panic!("point {key} is past the last cloud");
}

impl Styled for ScatterChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ScatterChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let layout = self.layout(radii(window.rem_size(), cx));
        let mut plotted = Plot::new(self.id.clone(), Pick::Dot, layout, self.tips());
        plotted.names = self.clouds.iter().map(|cloud| cloud.name.clone()).collect();
        plotted.voronoi = self.voronoi;
        plotted.format = self.format.clone();
        sized(&mut self.base, plotted, window, cx)
    }
}
