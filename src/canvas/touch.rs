use super::Viewport;

/// One active contact, with a stable backend id and canvas-local logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchContact {
    pub id: u64,
    pub at: (f32, f32),
}

/// Editing actions are previews until `End`; `Cancel` must discard them without committing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchAction {
    None,
    Begin { at: (f32, f32) },
    Drag { at: (f32, f32) },
    End { at: (f32, f32), tap: bool },
    Cancel,
    Viewport(Viewport),
}

#[derive(Clone, Copy, Debug)]
enum State {
    Idle,
    Single {
        contact: TouchContact,
        origin: (f32, f32),
        dragged: bool,
    },
    Pinch([TouchContact; 2]),
    Blocked,
}

/// Opt-in touch arbitration; desktop input handlers remain the owner's responsibility.
/// Use instead of GPUI's recognized gesture path, never alongside it for the same contacts.
#[derive(Debug)]
pub struct CanvasTouch {
    slop: f32,
    state: State,
}

impl CanvasTouch {
    /// `slop` is the host's drag threshold in logical pixels.
    pub fn new(slop: f32) -> Self {
        assert!(
            slop.is_finite() && slop >= 0.0,
            "invalid touch drag threshold"
        );
        Self {
            slop,
            state: State::Idle,
        }
    }

    /// Cancels editing on focus loss, OS cancellation, or lifecycle suspension.
    /// Supply an empty snapshot before accepting a new sequence; held fingers cannot restart edits.
    pub fn cancel(&mut self) -> TouchAction {
        self.state = State::Blocked;
        TouchAction::Cancel
    }

    /// Supply all active contacts after each change, including an empty snapshot after release.
    /// Deliver the final position before removing a contact; `End` uses its last supplied position.
    /// Invalid coordinates or duplicate ids cancel the sequence; three contacts block until release.
    /// Pinch scale is incremental (current distance / previous distance), never cumulative.
    pub fn update(&mut self, contacts: &[TouchContact], view: Viewport) -> TouchAction {
        if contacts.iter().enumerate().any(|(i, c)| {
            !c.at.0.is_finite()
                || !c.at.1.is_finite()
                || contacts[..i].iter().any(|other| other.id == c.id)
        }) {
            return self.cancel();
        }
        let previous = self.state;
        if contacts.is_empty() {
            self.state = State::Idle;
            return match previous {
                State::Single {
                    contact, dragged, ..
                } => TouchAction::End {
                    at: contact.at,
                    tap: !dragged,
                },
                _ => TouchAction::None,
            };
        }
        match (previous, contacts) {
            (State::Idle, [contact]) => {
                self.state = State::Single {
                    contact: *contact,
                    origin: contact.at,
                    dragged: false,
                };
                TouchAction::Begin { at: contact.at }
            }
            (
                State::Single {
                    contact: old,
                    origin,
                    dragged,
                },
                [contact],
            ) if old.id == contact.id => {
                let dragged = dragged || distance(origin, contact.at) > self.slop;
                self.state = State::Single {
                    contact: *contact,
                    origin,
                    dragged,
                };
                if dragged {
                    TouchAction::Drag { at: contact.at }
                } else {
                    TouchAction::None
                }
            }
            (State::Idle | State::Single { .. }, [a, b]) => {
                self.state = State::Pinch([*a, *b]);
                if matches!(previous, State::Single { .. }) {
                    TouchAction::Cancel
                } else {
                    TouchAction::None
                }
            }
            (State::Pinch(old), [a, b]) if same_pair(old, [*a, *b]) => {
                self.state = State::Pinch([*a, *b]);
                let before = distance(old[0].at, old[1].at);
                let after = distance(a.at, b.at);
                if !before.is_finite() || !after.is_finite() {
                    return self.cancel();
                }
                let from = center(old[0].at, old[1].at);
                let to = center(a.at, b.at);
                // Coincident contacts establish a fresh baseline without a zoom jump.
                let factor = if before > 0.0 && after > 0.0 {
                    after / before
                } else {
                    1.0
                };
                if !factor.is_finite() {
                    return self.cancel();
                }
                let next = view
                    .zoomed(factor, from)
                    .panned((to.0 - from.0, to.1 - from.1));
                if next.x.is_finite() && next.y.is_finite() {
                    TouchAction::Viewport(next)
                } else {
                    self.cancel()
                }
            }
            (State::Blocked, _) => TouchAction::None,
            _ => self.cancel(),
        }
    }

    /// Accept logical-pixel content movement only while idle; do not also feed synthesized wheel events.
    pub fn scroll(&self, delta: (f32, f32), view: Viewport) -> TouchAction {
        if !matches!(self.state, State::Idle) || !delta.0.is_finite() || !delta.1.is_finite() {
            return TouchAction::None;
        }
        let next = view.panned(delta);
        if next.x.is_finite() && next.y.is_finite() {
            TouchAction::Viewport(next)
        } else {
            TouchAction::None
        }
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn center(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 / 2.0 + b.0 / 2.0, a.1 / 2.0 + b.1 / 2.0)
}

fn same_pair(a: [TouchContact; 2], b: [TouchContact; 2]) -> bool {
    (a[0].id == b[0].id && a[1].id == b[1].id) || (a[0].id == b[1].id && a[1].id == b[0].id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact(id: u64, x: f32, y: f32) -> TouchContact {
        TouchContact { id, at: (x, y) }
    }

    fn view() -> Viewport {
        Viewport::new(10.0, 20.0, 1.0)
    }

    fn viewport(action: TouchAction) -> Viewport {
        match action {
            TouchAction::Viewport(view) => view,
            other => panic!("expected viewport, got {other:?}"),
        }
    }

    #[test]
    fn tap_and_drag_commit_once_only_on_release() {
        let mut touch = CanvasTouch::new(4.0);
        assert_eq!(
            touch.update(&[contact(1, 0.0, 0.0)], view()),
            TouchAction::Begin { at: (0.0, 0.0) }
        );
        assert_eq!(
            touch.update(&[contact(1, 3.0, 0.0)], view()),
            TouchAction::None
        );
        assert_eq!(
            touch.update(&[], view()),
            TouchAction::End {
                at: (3.0, 0.0),
                tap: true
            }
        );
        assert_eq!(touch.update(&[], view()), TouchAction::None);
        touch.update(&[contact(1, 0.0, 0.0)], view());
        assert_eq!(
            touch.update(&[contact(1, 5.0, 0.0)], view()),
            TouchAction::Drag { at: (5.0, 0.0) }
        );
        touch.update(&[contact(1, 0.0, 0.0)], view());
        assert_eq!(
            touch.update(&[], view()),
            TouchAction::End {
                at: (0.0, 0.0),
                tap: false
            }
        );
    }

    #[test]
    fn cancellation_never_releases_or_restarts_a_held_edit() {
        let mut touch = CanvasTouch::new(2.0);
        touch.update(&[contact(1, 0.0, 0.0)], view());
        assert_eq!(touch.cancel(), TouchAction::Cancel);
        assert_eq!(
            touch.update(&[contact(1, 50.0, 0.0)], view()),
            TouchAction::None
        );
        assert_eq!(touch.update(&[], view()), TouchAction::None);
        assert!(matches!(
            touch.update(&[contact(2, 0.0, 0.0)], view()),
            TouchAction::Begin { .. }
        ));
    }

    #[test]
    fn pinch_cancels_preview_and_preserves_the_moving_focal_point() {
        let mut touch = CanvasTouch::new(2.0);
        touch.update(&[contact(1, 0.0, 0.0)], view());
        assert_eq!(
            touch.update(&[contact(1, 0.0, 0.0), contact(2, 10.0, 0.0)], view()),
            TouchAction::Cancel
        );
        let held = view().to_canvas((5.0, 0.0));
        let near =
            viewport(touch.update(&[contact(2, 30.0, 10.0), contact(1, 10.0, 10.0)], view()));
        assert_eq!(near.zoom, 2.0);
        assert_eq!(near.to_canvas((20.0, 10.0)), held);
        let next = viewport(touch.update(&[contact(1, 0.0, 10.0), contact(2, 40.0, 10.0)], near));
        assert_eq!(next.zoom, 4.0);
        assert_eq!(next.to_canvas((20.0, 10.0)), held);
        assert_eq!(
            touch.update(&[contact(1, 0.0, 10.0)], next),
            TouchAction::Cancel
        );
        assert_eq!(
            touch.update(&[contact(1, 1.0, 10.0)], next),
            TouchAction::None
        );
        assert_eq!(touch.update(&[], next), TouchAction::None);
    }

    #[test]
    fn scroll_cannot_compete_with_touch() {
        let mut touch = CanvasTouch::new(2.0);
        assert_eq!(
            viewport(touch.scroll((2.0, 3.0), view())),
            view().panned((2.0, 3.0))
        );
        touch.update(&[contact(1, 0.0, 0.0)], view());
        assert_eq!(touch.scroll((2.0, 3.0), view()), TouchAction::None);
        touch.cancel();
        assert_eq!(touch.scroll((2.0, 3.0), view()), TouchAction::None);
    }

    #[test]
    fn malformed_contacts_cancel_without_a_commit() {
        for contacts in [
            vec![contact(1, f32::NAN, 0.0)],
            vec![contact(1, 0.0, 0.0), contact(1, 2.0, 0.0)],
        ] {
            let mut touch = CanvasTouch::new(2.0);
            touch.update(&[contact(1, 0.0, 0.0)], view());
            assert_eq!(touch.update(&contacts, view()), TouchAction::Cancel);
            assert_eq!(touch.update(&[], view()), TouchAction::None);
        }
    }

    #[test]
    fn pinch_limits_and_coincident_contacts_keep_the_anchor() {
        let mut touch = CanvasTouch::new(2.0);
        touch.update(&[contact(1, -1.0, 0.0), contact(2, 1.0, 0.0)], view());
        let next =
            viewport(touch.update(&[contact(1, -100.0, 0.0), contact(2, 100.0, 0.0)], view()));
        assert_eq!(next.zoom, Viewport::ZOOMS.1);
        assert_eq!(next.to_canvas((0.0, 0.0)), view().to_canvas((0.0, 0.0)));
        let flat = viewport(touch.update(&[contact(1, 0.0, 0.0), contact(2, 0.0, 0.0)], next));
        assert_eq!(flat, next);
        let apart = viewport(touch.update(&[contact(1, -1.0, 0.0), contact(2, 1.0, 0.0)], flat));
        assert_eq!(apart, flat);
        let far = viewport(touch.update(&[contact(1, -0.001, 0.0), contact(2, 0.001, 0.0)], apart));
        assert_eq!(far.zoom, Viewport::ZOOMS.0);
        assert_eq!(far.to_canvas((0.0, 0.0)), view().to_canvas((0.0, 0.0)));
    }

    #[test]
    fn extra_or_replaced_contacts_block_until_all_lift() {
        for contacts in [
            vec![contact(2, 0.0, 0.0)],
            vec![
                contact(1, 0.0, 0.0),
                contact(2, 2.0, 0.0),
                contact(3, 4.0, 0.0),
            ],
        ] {
            let mut touch = CanvasTouch::new(2.0);
            touch.update(&[contact(1, 0.0, 0.0)], view());
            assert_eq!(touch.update(&contacts, view()), TouchAction::Cancel);
            assert_eq!(
                touch.update(&[contact(2, 0.0, 0.0)], view()),
                TouchAction::None
            );
            assert_eq!(touch.update(&[], view()), TouchAction::None);
        }
    }

    #[test]
    fn extreme_coordinates_cancel_instead_of_panicking() {
        let mut touch = CanvasTouch::new(2.0);
        touch.update(&[contact(1, -1.0, 0.0), contact(2, 1.0, 0.0)], view());
        assert_eq!(
            touch.update(
                &[contact(1, -f32::MAX, 0.0), contact(2, f32::MAX, 0.0)],
                view()
            ),
            TouchAction::Cancel
        );
        assert_eq!(touch.update(&[], view()), TouchAction::None);
    }
}
