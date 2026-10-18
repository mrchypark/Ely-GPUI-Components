use gpui::{Pixels, Styled, px};

use super::Theme;

impl Theme {
    /// Preserve a resolved minimum while applying the optional touch floor.
    pub fn touch_minimum(&self, minimum: Pixels) -> Pixels {
        if self.touch_mode {
            minimum.max(px(48.))
        } else {
            minimum
        }
    }

    /// Sets minima, replacing existing minima; callers must preserve larger minimum sizes.
    pub fn touch_target<T: Styled>(&self, element: T) -> T {
        if self.touch_mode {
            let minimum = self.touch_minimum(Pixels::ZERO);
            element.min_w(minimum).min_h(minimum)
        } else {
            element
        }
    }

    /// Spacing between independent touch controls in logical pixels.
    pub fn touch_spacing(&self) -> Pixels {
        px(8.)
    }
}
