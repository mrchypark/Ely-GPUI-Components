# Mobile UI support (implementation preview)

## Current validation (2026-10-18)

Latest verified core overlay commits are app-compatible `09702cf` and registry
upstream `8563bc6`. The buttons/overlays/menus host suites passed respectively
12/39/47 for app and 20/39/65 for upstream, all exit 0. Evidence is
`frozen-batch-20261018-104222-48546-results.txt` and its six suite logs.
This includes bounded Popover/HoverCard behavior and the no-room correction;
it does not certify all overlay components or native input.

Earlier core commits `7e146ac` and `c8fe0a5` retain the separate
forms/lists/navigation evidence below; those suites were not rerun by this
documentation update. Host execution is recorded in task-local
`outputs/mobile-gallery-simulator-20261017/provenance/`:

| Source line | forms:: | lists:: | navigation:: |
| --- | --- | --- | --- |
| App-compatible | 144 PASS | 39 PASS | 35 PASS |
| Registry upstream | 147 PASS | 42 PASS | 35 PASS |

All six runs exited 0. Evidence:
`host-owned-fixed-20261018-101757-32699-results.txt`, the six matching suite
logs, and `host-owned-fixed-20261018-101757-32699-integrity.txt`. Source hashes
matched the six-suite snapshot and dependency pins stayed unchanged. These are
filtered host-suite totals, not counts of components or native acceptance.
They include overlapping and endpoint RangeSlider drags, form edge/disabled
behavior, visible Tabs context and EditorTabs close controls, Sortable move
controls, and swipe cancellation/parent containment.

Both galleries passed 10 host tests each in
`gallery-retry-20261018-105023-50471-ely-runtime.log` and
`gallery-retry-20261018-105023-50471-ely-upstream-runtime.log`; the matching
results file records build and runtime exit 0 for each. Each navigation test
draws all 44 registered pages at 390×844 logical pixels. Coverage also includes
catalog/assets, wheel scrolling of catalog and specimen with a stationary
header, four terminal replay tests, and fixture cleanup tests. Both cleanup
logs confirm automatic fixture removal without manual deletion.
This retry supersedes the earlier gallery failure in the frozen-batch run.
These are host checks, not 44 component certifications or visual/interaction
acceptance of every demo. Terminal replay does not certify a native PTY or shell.

The old native binary demonstrated only a catalog-to-buttons touch transition
and immediate render. It does not contain the newly verified core snapshot.
No new native trace PASS is established by this update. These documentation
edits are excluded from the current frozen native build snapshot.

Compound wrapper and gallery propagation of per-window safe-area/keyboard
`usable_bounds` remains unimplemented. Direct overlay bounds support and host
tests do not supply this wiring automatically. TooltipTrigger/HelpTooltip
touch-only disclosure also remains follow-up work; bounds must not be stored
globally in Theme.

The full component inventory and S0–S7 acceptance scope remain in
[component coverage](mobile-component-coverage.md). Export declaration groups
include models, helpers, functions and cfg-specific names; they are not
component counts. Native IME, rotation, multitouch, accessibility, lifecycle,
performance, OS services and category-wide acceptance remain separate work.

## Historical focused menu validation (2026-10-17)

The app-compatible Git GPUI branch passed 42 focused menu tests; the registry
upstream branch passed 60. These are separate runs, not full-suite certification.
On Android, the app-compatible integration APK SHA-256
`0098ddda661c0050794bba25efd6a099f7cac83dda659942d3d53dffc9ac5341`
passed menu opening, overlapping submenu swipe with stable background, Last item
selection (index 30, count 1), Home/resume, outside dismissal, and subsequent
canvas drag. The upstream registry branch was not the APK's dependency.

The previous APK leaked wheel events into underlying controls/canvas. Both source
lines now stop propagation at the outer menu panel after the rows handle their
scroll. Host regressions include an actual ancestor ScrollHandle, boundary/padding
containment and a closed-menu scrolling positive control. One earlier resume
observation (capture 13) remains unexplained and did not recur after cold restart;
it is not assigned a root cause. Cleanup stopped the test app and retained the APK.

This does not certify IME, rotation, multitouch, GPS/BLE, iOS runtime or all widgets.
IME backend ownership remains with gpui-mobile. Local evidence is retained under
`outputs/ely-menu-device-20261016/result.md` in the task workspace; artifacts are not
included in this library. Older check-only results below are historical snapshots,
not a statement that the focused menu tests remain unexecuted.
See [component roadmap](mobile-component-roadmap.md) for proposed follow-up work.


This branch adds opt-in UI policies to historical Ely `083ef856` with Zed GPUI
`1a28cff4b409169bac058bca40dfbfeb7621d19b`. Its initial check-only evidence did not establish a working mobile
application. See [mobile compatibility](mobile-build-compatibility.md) for dependency
identity and build evidence maintained by the build owner.

Ely remains a UI library. A host combines it with a mobile GPUI Platform; neither
Ely nor gpui-mobile core should depend on the other. GPS/BLE belong to application
services, outside the canvas and this library.

## Showcase

`mobile_showcase` is registered at `examples/mobile_showcase/main.rs`. Run
`cargo run --example mobile_showcase` on a desktop. `view.rs` is the shared view.
Resize across 600 logical pixels: navigation is inline on wide windows and opens
an existing bottom Sheet on compact windows. Map info opens Dialog. Both receive
usable bounds. Buttons, Input, FocusScope, ZoomControls and InfiniteCanvas reuse
existing Ely components and theme colors. Controls wrap and scroll.

Safe area and keyboard buttons supply **simulated** geometry. Replay pinch supplies
raw normalized contacts to CanvasTouch; it does not simulate OS delivery. Desktop
pan/wheel remains InfiniteCanvas's existing path. The launcher is desktop-only;
there is no mobile entry point. The build owner checks `view.rs` separately through
a standalone library probe, without compiling the desktop launcher.

The actual Input focus handle is connected to `reveal_when_focused_in_bounds`.
Its scroll frame shrinks with usable height, giving hidden content scroll extent.
A host must similarly resize its scroll frame or provide sufficient avoidance
padding: translating a field beyond the scroll handle's maximum cannot reveal it.
Test toggling the mock keyboard while Input retains focus, then Tab through controls.
The example does not handle floating keyboards or native IME.

## Layout contract

`usable_bounds(viewport, ViewportInsets, keyboard_bottom)` receives logical window
coordinates. The keyboard value is occlusion from the **original viewport bottom**,
including overlapping safe area; safe bottom and keyboard occlusion use their
maximum, not their sum. Pass zero when the OS already resized the viewport.

The sibling backend's `WindowInsets` provides `safe_area: Edges<Pixels>` and
`keyboard: Option<Bounds<Pixels>>`. Copy edges explicitly. Only a bottom-docked
keyboard covering the content width can be reduced to bottom occlusion. Floating
keyboard rectangles require application-specific avoidance; converting every
rectangle to bottom height would incorrectly discard usable space.

Use `is_compact` on available width, not OS identity. `Dialog::usable_bounds` and
`Sheet::usable_bounds` opt into placement constraints. Their default desktop API
remains available. The existing internal `bring_into_view` / `reveal_when_focused`
behavior is extended through public `reveal_when_focused_in_bounds`; provide
painted field bounds and a real scroll owner when using lower-level reveal helpers.

## Backend/application input adapter

Pass the same available rectangle to `Menu::usable_bounds(available)` on the root
menu. Dropdown, context and searchable menus and all their submenus share it;
filtering preserves it. Rebuild with updated bounds when keyboard or viewport
geometry changes. Explicit bounds cap width and scrolling height and clamp
placement using the current frame's actual layout size. Default desktop placement
is unchanged. The mobile showcase demonstrates a nested menu with simulated
insets. Core Android menu touch acceptance is recorded above; keyboard acceptance remains separate.

The inspected sibling contract is `gpui-mobile/src/mobile_events.rs`:
`MobileEvent::Touch` carries one GPUI TouchEvent with Started/Moved/Ended/Cancelled
and window-local logical coordinates. CanvasTouch instead consumes a snapshot of
all active `TouchContact`s in **canvas-local** logical pixels.

The integration app must maintain a per-window contact map, subtract the actual
painted canvas origin (including safe area, toolbar and scrolling), and submit a
snapshot after each change. Before removing an Ended contact, submit its final
position; then remove it and submit the remaining snapshot. On cancellation,
focus loss, pause or surface loss, discard editing previews, call `cancel`, clear
the map and supply an empty snapshot. A cancelled release must never commit edits.

Choose raw contacts **or** the recognized GPUI pinch path for each gesture.
The showcase's Replay pinch button uses raw contacts only.
CanvasTouch computes incremental distance ratios internally. Backend
`MobileEvent::Pinch.scale` is cumulative since Started and cannot be passed as an
incremental factor repeatedly. CanvasTouch returns Begin/Drag previews, End for
commit, Cancel for discard, and Viewport updates. The owner must apply them to its
editing/history policy; a second contact cancels the single-contact preview.

InfiniteCanvas now handles recognized GPUI `PinchEvent` through `on_pinch`, zooming
by `1 + delta` around the canvas-local anchor, stopping propagation and clearing
stale pan state. The GPUI raw recognizer does not generate pinch from raw contacts:
the backend must emit recognized PinchEvent, or the application uses CanvasTouch.
The sibling observer's cumulative native pinch scale is not a PinchEvent delta.
An adapter must track the previous cumulative scale and emit
`delta = current_scale / previous_scale - 1`, resetting the baseline to 1 at
Started and discarding it at Ended/Cancelled. Validate finite positive scales and
translate coordinates consistently. Do not feed both paths for the same gesture.
Three `canvas::tests::pinch` UI tests cover this path, including malformed-event
pan preservation; they compile in the final iOS check but were not executed.

`set_mobile_event_handler` is an observer and does **not** consume the existing
GPUI synthesized mouse/scroll path. An application must arrange exclusive input
ownership before wiring raw touch to an editing canvas. Otherwise one gesture may
pan, zoom or commit twice. No such native suppression adapter is supplied here.

## Component scope

| Area | Implemented surface | Still requires integration/runtime evidence |
| --- | --- | --- |
| Buttons | Opt-in touch_mode for supported targets; see usage below | Native sizing, accessibility and OS tap synthesis beyond prior menu evidence |
| Input | Real focus reveal with changing usable bounds | Native IME, caret/selection, floating keyboard |
| List/scroll | Existing scroll container; focus reveal helper | Inertia, nested scrolling, capture outside bounds |
| Dialog | Opt-in usable bounds; existing focus policy | Keyboard and focus return on a real device |
| Sheet/navigation | Opt-in bounds, focus trap/return and body scroll | Broader device accessibility and keyboard checks |
| Menu | Usable bounds, scrolling and wheel containment | Core Android menu PASS above; long press and accessibility remain unverified |
| Canvas | Recognized GPUI PinchEvent handler plus opt-in normalized contacts | Backend adapter, edit/history adapter, duplicate event suppression |
| Desktop | Existing APIs retained | Full regression suite and visual checks pending |

## patch-map compatibility

patch-map re-exports Ely `canvas::{Frame, Viewport}` and uses `fitting`, pan/zoom
and coordinate conversion. Its own down/moved/up edit state machine still needs
an application adapter for CanvasTouch actions. `layout::Viewport` is a different
component; changing it alone cannot adapt patch-map.

The fixed **144px sidebar belongs to the patch-native application and is unchanged**.
The showcase's compact navigation does not modify that application. Neither
patch-native nor gpui-mobile is edited by this example/documentation change.

The example tests `patch_map_viewport_contract_keeps_pinch_anchor` and
`safe_origin_is_removed_before_canvas_hit_testing` cover API/math compatibility,
not patch-map integration, editing history, linking or rendering.

## Historical initial implementation evidence

Implementation commits: canvas `141f3b4`, layout `870a1b5`, platform/showcase
`1f215e9` on feature/mobile-ui.
Logs live in `work/build-logs` in the task directory. Results are separate evidence
levels; compiler success does not establish linking, execution or performance.

| Check | Result and scope |
| --- | --- |
| Final iOS library/tests with test-support | Exit 0, 30.54s, no warnings; check-final-ios-tests.log; compilation only, including 3 native pinch UI tests |
| Final shared-view iOS library | Exit 0, 13.81s; check-final-showcase-ios.log; no link/run |
| Final complete patch-map iOS library | Exit 0, 1.61s; check-final-patchmap-ios.log; no mobile backend included |
| Final desktop library/showcase | Exit 0, 1m54s; check-final-desktop.log; upstream block 0.1.6 future-incompatibility warning only |
| Original patch-map source | All 18 source/asset/example Git blob hashes match 4c2b5ea15620917b872a3d18d8f9b7e2d875e993 |
| Independent rustc tests | 12 passed, 0 failed; pure-canvas-tests.log; distinct from GPUI UI test execution |
| Final formatting/diff | cargo fmt --check and git diff --check: exit 0 |
| Final Android library | Exit 0, 1m26s; check-final-android.log; final source type-check only |
| All existing desktop examples | Exit 0, 5.27s; check-final-examples.log; upstream block 0.1.6 future-incompatibility warning |
| Final iOS Clippy | Exit 0, 41.81s; --lib --tests, test-support, -D warnings; clippy-final-ios.log |
| Initial test execution and runtime status | At that initial snapshot GPUI tests were not executed; later focused menu and Android results supersede this status as described above |

The source identity check uses `work/patch-map-check/verify-source.sh`. Only the
scratch manifest adapts workspace dependencies. Shared-view verification uses
`work/showcase-check/Cargo.toml`, importing the example view.rs directly. Both
probes retain the common GPUI source; they do not combine a mobile backend with
Ely and patch-map. Historical combined-backend API failures therefore require
separate integration verification.

The desktop command is `cargo run --example mobile_showcase`; running and capturing
it is still a separate task from the completed check. Coordinate all additional
Cargo checks with the build owner and shared-cache disk budget. Manual validation
still includes 280px width, large text, light/dark captures, focus restoration,
cancellation outside bounds, native keyboard/caret behavior and device input.
Windows/Linux builds and native pointer capture outside bounds remain unverified.

## Upstream path

Canonical Gitea main was reported at `5a53c08c`, using gpui-pre 0.3.8. This branch
intentionally preserves historical GPUI. Future submission requires a coordinated
forward-port across Ely and gpui-mobile, one GPUI source/revision, compatible wgpu,
Cargo tree verification and renewed integration checks. No unilateral pin upgrade.
CONTRIBUTING requests an issue before large changes and light/dark screenshots.
The task's PR draft is local only; nothing was submitted upstream.

## Process evidence

Yeoul lookup failed with a storage error; no memory writes occurred. Evidence is
retained here and in task-local logs. `work/team-run.json` records team evaluation.
A reusable POSIX verification pipeline with a disk-capacity gate is a follow-up
candidate only; no new skill was installed. The Python session evaluator was not
run under the user's scripting policy.
## Opt-in touch sizing

After Ely initialization, enable the policy with
`Theme::update(cx, |theme| theme.touch_mode = true)` using
`ely_gpui_component::theme::Theme`. The default is `false`; setting it back to
`false` restores the existing desktop sizing policy. ControlSize and Density
values themselves are unchanged.

Supported targets are Button (including ButtonGroup/SplitButton reuse), IconButton,
ConfirmButton, CopyButton, ToggleButton/ToggleGroup, SegmentedControl options and
QuickAction rows. Touch mode applies a minimum 48 logical pixels in each axis;
ActionBar and ToggleGroup use 8 logical pixels between independent controls.
Joined button/segment controls retain their joined presentation. Decorative
separators are excluded. Toggle minimum widths preserve larger sizes resolved
from the current window rem size, rather than replacing them with 48 pixels.

Custom controls can apply `theme.touch_target(element)` before their custom
minimum constraints. This helper replaces existing style minima; it does not
merge arbitrary rem/percentage constraints. Resolve an existing minimum to logical
pixels and pass it through `theme.touch_minimum(minimum)` when preserving it.

The app-compatible button suite passed 6 tests after the large-rem toggle fix;
independent review found no remaining blocker. The upstream buttons filter passed
14 tests (6 core button tests plus 8 menu-button tests), exit 0; the app run also
exited 0. These are separate suite totals, not extra tests to add again.
This is host evidence only: no new native
or simulator touch-sizing acceptance is claimed. Selected forms, list gestures
and navigation controls now have the host evidence recorded above; remaining
controls and native behavior still need migration or validation. Enabling
touch_mode is not whole-library mobile certification.
