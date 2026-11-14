use gpui::{AbsoluteLength, DefiniteLength, Length, Pixels, Styled, px};

use super::Theme;

impl Theme {
    /// Width below which a touch master-detail view shows one pane.
    pub fn touch_compact_width(&self) -> Pixels {
        px(600.)
    }

    /// Preserve a resolved minimum while applying the optional touch floor.
    pub fn touch_minimum(&self, minimum: Pixels) -> Pixels {
        if self.touch_mode {
            minimum.max(px(48.))
        } else {
            minimum
        }
    }

    /// Apply the 48px hit floor; see docs/touch-targets.md for its limits.
    pub fn touch_target<T: Styled>(&self, mut element: T) -> T {
        if self.touch_mode {
            let minimum = self.touch_minimum(Pixels::ZERO);
            let style = element.style();
            style.min_size.width = touch_floor(style.min_size.width, minimum);
            style.min_size.height = touch_floor(style.min_size.height, minimum);
        }
        element
    }

    /// Spacing between independent touch controls in logical pixels.
    pub fn touch_spacing(&self) -> Pixels {
        px(8.)
    }
}

/// The floor in rems, for a minimum given in rems.
const FLOOR_REMS: f32 = 3.0;

/// Returns the length to use for a minimum size once the touch floor is applied.
fn touch_floor(current: Option<Length>, floor: Pixels) -> Option<Length> {
    match current {
        None => Some(Length::Definite(DefiniteLength::Absolute(
            AbsoluteLength::Pixels(floor),
        ))),
        Some(Length::Auto) => Some(Length::Definite(DefiniteLength::Absolute(
            AbsoluteLength::Pixels(floor),
        ))),
        Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels)))) => {
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(pixels.max(floor)),
            )))
        }
        // Rem minima can be compared against the floor in rems.
        Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Rems(rems)))) => {
            if rems.0 >= FLOOR_REMS {
                Some(Length::Definite(DefiniteLength::Absolute(
                    AbsoluteLength::Rems(rems),
                )))
            } else {
                Some(Length::Definite(DefiniteLength::Absolute(
                    AbsoluteLength::Rems(gpui::rems(FLOOR_REMS)),
                )))
            }
        }
        // Fractional minima depend on the parent and cannot be compared here.
        Some(other) => Some(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_auto_minimum_becomes_the_floor() {
        assert_eq!(
            touch_floor(None, px(48.)),
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(48.))
            )))
        );
    }

    #[test]
    fn a_larger_pixel_minimum_survives() {
        let larger = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(96.))));
        assert_eq!(touch_floor(Some(larger), px(48.)), Some(larger));
    }

    #[test]
    fn a_smaller_pixel_minimum_grows_to_the_floor() {
        let smaller = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(20.))));
        assert_eq!(
            touch_floor(Some(smaller), px(48.)),
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(48.))
            )))
        );
    }

    #[test]
    fn a_rem_minimum_grows_to_the_floor() {
        let rem = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Rems(gpui::rems(
            2.0,
        ))));
        assert_eq!(
            touch_floor(Some(rem), px(48.)),
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Rems(gpui::rems(3.0))
            )))
        );
    }

    #[test]
    fn a_larger_rem_minimum_survives() {
        let rem = Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Rems(gpui::rems(
            4.0,
        ))));
        assert_eq!(touch_floor(Some(rem), px(48.)), Some(rem));
    }

    #[test]
    fn a_fractional_minimum_is_left_alone() {
        let fraction = Length::Definite(DefiniteLength::Fraction(0.5));
        assert_eq!(touch_floor(Some(fraction), px(48.)), Some(fraction));
    }
}
