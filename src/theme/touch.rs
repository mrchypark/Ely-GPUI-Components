use gpui::{AbsoluteLength, DefiniteLength, Length, Pixels, Styled, px};

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

    /// Applies the 48 pixel touch floor without shrinking a larger existing minimum.
    ///
    /// The floor is a *hit area* guarantee, not a layout guarantee. It is meant for
    /// controls whose visual size carries no spatial meaning: buttons, list rows,
    /// nav items, inputs, tabs, map markers and clusters, canvas editing handles.
    ///
    /// It is deliberately **not** applied to elements whose size or position is the
    /// encoding: map pin tips, route and line strokes, heat-map cells, chart glyphs,
    /// selection overlays, and canvas content. Enlarging those changes what the
    /// user sees rather than what they can hit, so they should keep an explicit
    /// `min_size` (or a padded transparent hit shape) instead.
    ///
    /// Existing minima are preserved: a minimum larger than the floor wins. A `rem`
    /// minimum is compared in rems, using the crate's own convention of sixteen
    /// pixels to a rem, so a two rem row becomes three rems rather than losing the
    /// floor. A fractional minimum is left untouched, because its resolved size
    /// depends on the parent, which is not known here; callers that know the
    /// resolved value should apply [`Theme::touch_minimum`] to it directly.
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
        // A rem minimum is a fixed size in this crate, so it can be compared with
        // the floor in rems and raised to it when it is smaller.
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
        // A fractional minimum is resolved against the parent, so it cannot be
        // compared with a floor here; keep the caller's intent.
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
