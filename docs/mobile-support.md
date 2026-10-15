# Mobile layout and input contract

This document specifies the adapter contract for the forward port. It is not evidence of device support. See [the compatibility matrix](mobile-build-compatibility.md) for dependency identities and validation status.

## Geometry and focus

The application supplies window geometry in logical window pixels, not physical framebuffer pixels. Normalize safe-area insets and docked keyboard occlusion to the current viewport, including any resize already applied by the backend. Both exclusions must describe the remaining overlap with that same viewport. Where their bottom exclusions overlap, reserve `max(safe_bottom, keyboard_bottom_occlusion)`, never their sum. For example, a safe bottom of 34 and keyboard occlusion of 300 reserve 300, not 334. If the resized viewport already excludes the keyboard, its remaining keyboard occlusion is zero; do not subtract the original keyboard height again.

A keyboard rectangle may become a bottom inset only when it covers the full viewport width and docks to its bottom edge, after normalization to the current viewport. Floating or split keyboard rectangles are unsupported by this conversion; do not treat their height as a bottom inset. The example-only conversion helper and tests are implemented. Review found no functional issues for expected finite, valid bounds: coverage includes a nonzero origin, full-width bottom dock, maximum overlapping exclusion, hidden keyboard, and floating/partial rectangles. The scratch shared-view tests compiled successfully for iOS (exit 0); their execution remains unverified. This mock helper demonstrates the integration contract, not a native backend; OS normalization remains unverified. Insets alone do not prove focused-field visibility: the integration must connect current focused-field/caret bounds to its scroll owner, reveal the field after layout changes, and preserve focus restoration when overlays close.

Choose compact navigation and overlay placement from available bounds, not OS identity. Reuse existing Dialog and Sheet behavior; clamp their size and placement to the available region. Preserve desktop keyboard behavior and focus ownership. Test 280px width, large text, light/dark appearance, rotation, keyboard transitions and overlay dismissal separately from compilation.

## Canvas coordinates and gesture ownership

Canvas input uses canvas-local logical pixels. Subtract the rendered canvas origin in window coordinates exactly once before hit-testing or applying a pinch anchor. Include safe-area/layout offsets in that origin; do not subtract them a second time. Apply the returned viewport and pass the updated viewport to the next incremental update.

Select one pinch owner for a contact stream:

- Raw contacts: send stable contact IDs and current local positions to `CanvasTouch`. On OS Ended, first update the snapshot with that contact's final position, then remove the ended contact and deliver the remaining snapshot (empty only when no contacts remain). Begin/Drag are previews; commit only on End. Remaining fingers cannot restart editing after a pinch until all contacts lift.
- Recognized gestures: let the backend emit GPUI pinch events for `InfiniteCanvas`. Source review confirmed the registry API retains the incremental multiplier `1.0 + event.delta`. The handler converts the window anchor to canvas-local coordinates. Native pinch dispatch is hit-tested, not pointer capture; the adapter must account for motion and termination outside bounds.

The registry GPUI raw-touch recognizer ignores extra contacts and does not recognize pinch. Native pinch support therefore does not automatically provide multi-touch support from raw OS contacts.

Do not feed both routes for the same contacts. The application/backend adapter owns suppression of synthetic mouse clicks, drags and wheel events for contacts already consumed as touch or recognized pinch. Stopping event propagation alone does not deduplicate independently synthesized events. Assign scroll/pan ownership once so a single physical movement does not move the viewport twice.

Two cancellation paths have different ownership:

- `CanvasTouch::update` returning Cancel when a second finger arrives means discard the edit preview only. Keep the contacts and internal pinch state. Do not call `.cancel()`, clear contacts, or inject an empty snapshot in response to this action; doing so would abort the pinch that just started.
- External OS Cancelled, pause/suspension, surface loss, or capture/focus loss requires `.cancel()`, discarding the edit preview, clearing the adapter's contacts, and delivering an empty snapshot. Do not route this through ordinary Ended handling or commit the preview.

The adapter owns pointer capture, focus assignment, native event translation and external cancellation. Neither cancellation path clears an unrelated selection. Invalid pinch data must not destroy an active desktop pan. Validate release outside bounds, second-contact arbitration, cancellation, anchor preservation, zoom limits and synthetic-event suppression with the actual host. This contract does not claim that a raw OS adapter has been implemented in Ely.

## Scope and evidence limits

The showcase demonstrates integration policy with simulated safe-area/keyboard data and gesture replay. A desktop launcher or mobile type-check entry point is not an iOS/Android host. Buttons, inputs, lists, menus, scrolling, dialogs and sheets still require real touch, keyboard/IME, accessibility and focus testing. Hover-dependent affordances and small controls need device review. GPS/BLE and domain-specific map editing belong outside Ely.

The patch-map application uses `canvas::Viewport`; it does not gain mobile support merely because `layout::Viewport` changes. Application editing/history, backend rendering, linking and performance require independent integration evidence. The registry candidate does not change the application's pinned GPUI contract.
