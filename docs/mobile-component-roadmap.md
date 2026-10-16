# Mobile component roadmap

Implemented: reuse responsive layout and `usable_bounds` for Dialog, Sheet and
Menu; Sheet focus/scroll behavior; bounded scrollable submenu placement and wheel
containment; normalized canvas contacts and recognized pinch support. These APIs
preserve desktop defaults. The app-compatible menu exposes `on_open_change` for
integration gesture ownership. Android core-menu acceptance is recorded in
[mobile support](mobile-support.md); it is not acceptance of every component.

The following are proposals, not implemented APIs or delivery commitments:

| Priority | Next work | Reuse and acceptance boundary |
| --- | --- | --- |
| 1 | Touch hit-target policy | Extend existing ControlSize/Density; Lg + Comfortable is 36 logical px at the default rem size, so density alone is not a minimum-hit-target guarantee. Check small controls and spacing on devices. |
| 1 | Form keyboard integration | Reuse input focus/reveal and usable bounds; validate caret, selection, keyboard changes and IME with the platform owner. |
| 1 | Accessibility | Audit existing roles, focus order, labels, scaling and screen-reader behavior; retain desktop keyboard operation. |
| 2 | Action sheet presentation | Reuse Sheet and existing actions; define dismissal and focus restoration before adding API. |
| 2 | Bottom navigation | Adapt existing navigation/Tabs to available width and safe area; avoid a parallel navigation implementation. |
| 2 | List gestures | Reuse SwipeableListItem, VirtualList and InfiniteList; verify scroll/swipe arbitration, cancellation and long-list behavior. |

Ely owns component state, layout, focus policy and propagation containment.
gpui-mobile owns platform windows, rendering, OS event translation, IME transport
and lifecycle/inset reporting. The integration app normalizes geometry and assigns
one gesture owner; GPS/BLE and domain state remain application services. Neither
library should acquire a dependency on the other merely to implement this policy.

The Git GPUI compatibility branch and registry upstream branch retain different
pins. Do not infer native compatibility from matching component APIs or merge
those dependency graphs without a separate integration check.
