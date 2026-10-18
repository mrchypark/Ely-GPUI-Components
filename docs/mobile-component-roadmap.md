# Mobile component roadmap

Implemented: reuse responsive layout and `usable_bounds` for Dialog, Sheet and
Menu; Sheet focus/scroll behavior; bounded scrollable submenu placement and wheel
containment; normalized canvas contacts and recognized pinch support. These APIs
preserve desktop defaults. The app-compatible menu exposes `on_open_change` for
integration gesture ownership. Android core-menu acceptance is recorded in
[mobile support](mobile-support.md); it is not acceptance of every component.

Latest core overlay evidence (2026-10-18): app `09702cf` and upstream
`8563bc6`. The `frozen-batch-20261018-104222-48546` buttons/overlays/menus
logs record app 12/39/47 PASS and upstream 20/39/65 PASS, all exit 0.
Popover/HoverCard bounds, scrolling, touch disclosure and no-room behavior
have focused host coverage. This is not category-wide or native acceptance.

Earlier core evidence: app `7e146ac` and upstream `c8fe0a5`.
The `host-owned-fixed-20261018-101757-32699` provenance run passed
forms/lists/navigation at 144/39/35 for app and 147/42/35 for upstream,
with unchanged source hashes and pins. These are filtered host tests, not
component counts or blanket category acceptance. Selected form hit targets,
overlapping/endpoint RangeSlider drags, Tabs context controls, EditorTabs close
controls, Sortable move buttons and swipe containment are implemented.

Both galleries: 10 host tests PASS each; all 44 registered pages draw at
390×844 in each navigation test. The retry also checks wheel scrolling,
four terminal replay cases, catalog/assets and fixture cleanup. Both automatic
cleanup checks passed without manual removal. Evidence is the
`gallery-retry-20261018-105023-50471` runtime/results/cleanup logs under
task-local `outputs/mobile-gallery-simulator-20261017/provenance/`.
The retry supersedes the earlier frozen-batch gallery failure. This does not
verify every component, interaction or visual state, or native PTY/shell.
The old native binary only demonstrated catalog-to-buttons touch navigation
and immediate render. No new native trace PASS is established. These docs
are excluded from the current frozen native snapshot.

The full [coverage inventory and S0–S7 scenarios](mobile-component-coverage.md)
remain in scope. Export groups include nonvisual APIs and are not component
counts. The following are remaining work, not delivery commitments:

| Priority | Next work | Reuse and acceptance boundary |
| --- | --- | --- |
| 1 | Extend touch-policy coverage | Buttons and selected forms/lists/navigation controls are implemented and host-tested; audit remaining exports individually and verify device hit targets and spacing. |
| 1 | Form keyboard integration | Reuse input focus/reveal and usable bounds; validate caret, selection, keyboard changes and IME with the platform owner. |
| 1 | Accessibility | Audit existing roles, focus order, labels, scaling and screen-reader behavior; retain desktop keyboard operation. |
| 1 | Compound overlay bounds propagation | Still unimplemented: forward per-window safe-area/keyboard usable bounds through existing wrappers and gallery callers; do not store bounds globally in Theme. Direct Popover/HoverCard tests do not cover this wiring. |
| 1 | Tooltip touch disclosure | TooltipTrigger/HelpTooltip still need a touch-only access path through their actual callers. |
| 2 | Action sheet integration | Core host coverage is recorded above; verify actual caller geometry, dismissal/focus and native long-list interaction separately. |
| 2 | Bottom navigation | Adapt existing navigation/Tabs to available width and safe area; avoid a parallel navigation implementation. |
| 2 | List gestures | SwipeableListItem containment/cancel and Sortable move controls have host regressions; verify native arbitration, cancellation, VirtualList/InfiniteList and long-list behavior separately. |

Ely owns component state, layout, focus policy and propagation containment.
gpui-mobile owns platform windows, rendering, OS event translation, IME transport
and lifecycle/inset reporting. The integration app normalizes geometry and assigns
one gesture owner; GPS/BLE and domain state remain application services. Neither
library should acquire a dependency on the other merely to implement this policy.

The Git GPUI compatibility branch and registry upstream branch retain different
pins. Do not infer native compatibility from matching component APIs or merge
those dependency graphs without a separate integration check.

Implemented touch foundation: `Theme::update(cx, |theme| theme.touch_mode = true)`
opts supported button targets into minimum 48 logical px and ActionBar/ToggleGroup
spacing into 8 logical px. Default `false` retains desktop sizing. Button,
IconButton, ConfirmButton, CopyButton, ToggleButton/ToggleGroup, SegmentedControl
and QuickAction reuse the policy; ButtonGroup/SplitButton reuse Button. Larger
rem-based toggle minimums are preserved. Historical app host buttons: 6 PASS after correction;
upstream buttons filter: 14 PASS (6 core + 8 menu-button), both runs exit 0.
These are separate totals. See [usage and helper contract](mobile-support.md#opt-in-touch-sizing).
No new native acceptance follows from these host checks; remaining component
coverage and accessibility are still planned work.
