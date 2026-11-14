use super::*;
use crate::theme::Palette;

pub(super) fn paint_horizon_canvas(
    rows: Vec<HorizonRow>,
    xs: Vec<f32>,
    rect: Rect,
    bands: usize,
    hovered_rc: Option<(usize, usize)>,
    is_pinned: bool,
    brush: Option<(f32, f32)>,
    total_cols: usize,
    hairline: Pixels,
    colors: Palette,
) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            let pos_color = colors.chart[0];
            let neg_color = colors.danger;
            let grid = colors.border.opacity(0.6);

            for (r_ix, row) in rows.iter().enumerate() {
                let row_hovered = hovered_rc.is_some_and(|(hr, _)| hr == r_ix);
                for patch in &row.patches {
                    if patch.points.len() < 3 {
                        continue;
                    }
                    let mut path = PathBuilder::fill();
                    path.move_to(at(origin, patch.points[0]));
                    for &pt in &patch.points[1..] {
                        path.line_to(at(origin, pt));
                    }
                    path.close();
                    let base_color = if patch.positive { pos_color } else { neg_color };
                    let alpha = if row_hovered {
                        (band_alpha(patch.band, bands) + 0.08).min(0.95)
                    } else {
                        band_alpha(patch.band, bands)
                    };
                    finish(path, base_color.opacity(alpha), window);
                }
                window.paint_quad(fill(
                    gpui::Bounds::new(
                        at(origin, (row.rect.x, row.rect.y + row.rect.h)),
                        size(Pixels::from(row.rect.w), hairline),
                    ),
                    if row_hovered {
                        colors.border_strong
                    } else {
                        grid
                    },
                ));
            }
            if let Some((_, col_ix)) = hovered_rc
                && let Some(&x_px) = xs.get(col_ix)
            {
                window.paint_quad(fill(
                    gpui::Bounds::new(
                        at(origin, (x_px, rect.y)),
                        size(hairline, Pixels::from(rect.h)),
                    ),
                    if is_pinned {
                        colors.fg
                    } else {
                        colors.border_strong
                    },
                ));
            }
            if let Some((a, b)) = brush
                && total_cols >= 3
                && (b - a).abs() > 4.0
            {
                let band = Rect {
                    x: a.min(b),
                    y: rect.y,
                    w: (a - b).abs(),
                    h: rect.h,
                };
                window.paint_quad(fill(place(origin, band), colors.selection));
            }
        },
    )
}
