use gpui::Hsla;

use super::GradientStop;
use crate::theme::Mix;

pub(crate) fn color_at(stops: &[GradientStop], t: f32) -> Hsla {
    let first = stops.first().expect("a gradient has stops");
    if t <= first.at {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if t <= to.at {
            let span = (to.at - from.at).max(f32::EPSILON);
            return from.color.mix(&to.color, (t - from.at) / span);
        }
    }
    stops.last().expect("a gradient has stops").color
}

/// `stops` with `ix` moved to `t`, still sorted, and where `ix` landed.
pub(crate) fn moved(stops: &[GradientStop], ix: usize, t: f32) -> (Vec<GradientStop>, usize) {
    let at = t.clamp(0.0, 1.0);
    if at == stops[ix].at {
        return (stops.to_vec(), ix);
    }
    let mut next = stops.to_vec();
    let mut stop = next.remove(ix);
    stop.at = at;
    let landed = next.partition_point(|other| other.at <= stop.at);
    next.insert(landed, stop);
    (next, landed)
}

/// `stops` with a new stop at `t`, colored as the gradient is there.
pub(crate) fn added(stops: &[GradientStop], t: f32) -> (Vec<GradientStop>, usize) {
    let stop = GradientStop {
        at: t.clamp(0.0, 1.0),
        color: color_at(stops, t),
    };
    let mut next = stops.to_vec();
    let landed = next.partition_point(|other| other.at <= stop.at);
    next.insert(landed, stop);
    (next, landed)
}

/// `stops` without `ix`, with the nearest remaining stop chosen.
pub(crate) fn removed(stops: &[GradientStop], ix: usize) -> (Vec<GradientStop>, usize) {
    let mut next = stops.to_vec();
    next.remove(ix);
    (next, ix.saturating_sub(1))
}
