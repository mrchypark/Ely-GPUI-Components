# Mobile UI support (implementation preview)

This branch adds opt-in UI policies to historical Ely `083ef856` with Zed GPUI
`1a28cff4b409169bac058bca40dfbfeb7621d19b`. It does not establish a working mobile
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
| Buttons | Existing theme controls in wrapping toolbar | Touch target sizes, accessibility, OS tap synthesis |
| Input | Real focus reveal with changing usable bounds | Native IME, caret/selection, floating keyboard |
| List/scroll | Existing scroll container; focus reveal helper | Inertia, nested scrolling, capture outside bounds |
| Dialog | Opt-in usable bounds; existing focus policy | Keyboard and focus return on a real device |
| Sheet/navigation | Opt-in bounds; compact bottom sheet example | Sheet focus is not equivalent to Dialog's focus trap |
| Menu | Existing desktop implementation | Touch dismissal, long press, safe-area popup placement |
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

## Current evidence

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
| Test execution, link, simulator/device, visuals, native IME, FPS | GPUI tests not executed; runtime/visual evidence not established |

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
