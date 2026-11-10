# Mobile component audit — 2026-11-10

This records the local candidate in `feature/mobile-fast`, not a published release. The gallery catalog contains 44 pages. The automated gallery test draws every registered story; drawing a story does not establish that every control works on a device. See `mobile-component-coverage.md` for the component inventory.

## Changes made

- Mobile master/detail layouts stack on narrow touch windows and return to a side-by-side layout at wider sizes. Nested layouts may provide their available width explicitly.
- Form input owners can be reported app-wide, avoiding stale Android IME ownership when a form is redrawn. An individual input can still override the reporter. A host must register the reporter; see the gpui-mobile Ely integration example. The published Ely revision currently pinned by that example predates this API.
- Android and iOS platform windows expose their latest safe-area and keyboard insets to GPUI and notify the window when they change. The gallery intersects its content bounds with the visible window. Android NativeActivity insets use the rendering window, and an inset change requests a cache-bypassing frame.
- Select menus use visible bounds and block pointer events through their background. Dialogs block input behind the full window and preserve a usable action area on short screens. Lightboxes accept usable bounds; gallery examples pass them through.
- Common floating and navigation controls use a 48-point minimum touch target in touch mode.
- The mobile gallery test target is available under `test-support` and covers width changes across phone and tablet sizes.

## Verification

- Ely library: 1,971 tests passed with Rust 1.99.0 and `test-support`.
- Mobile gallery: 39 tests passed, including drawing all catalog stories and reactive width changes.
- Android emulator: the local candidate built and launched. Gboard entered text in two fields without mixing the values after a focus change. After the inset fix, the profile dialog moved above Gboard as soon as the keyboard appeared; tapping Save closed the dialog. The `insets_surface` Android instrumentation scenario passed. A resized emulator was used for tablet layout inspection.
- iOS: the local candidate built and launched on iPhone and iPad simulators. Forms were visible with narrow and wide layouts. A separate XCUITest drove the Korean two-set software keyboard: the first field showed `안녕`, the second showed `나`, and both values remained correct after returning focus to the first field. The result was checked on screenshots, not through an accessibility value assertion.
- The native test host points to the local Ely and gpui-mobile sources. These checks do not establish that downstream applications using existing pinned Git revisions include these changes.

## Remaining work, ordered by impact

1. Android landscape and a 900×1600 emulated display were tested with Gboard. Compact dialogs keep one input and the action row visible; the other input can be reached by scrolling. Focusing the other field can reset the scroll and hide it again, so automatic focused-field reveal remains unresolved.
2. On iPad 9, a split keyboard was enabled and Korean composition was entered using keys on both halves. On iPad Pro, moving a floating keyboard over the focused field obscured it; the gallery does not yet consume the floating rectangle. Split keyboard center-gap behavior and iOS keyboard dismissal need further validation.
3. Exercise controls individually on phone and tablet in portrait and landscape. Gallery draw coverage does not verify gestures, accessibility, long content, loading, error states or each component's interaction semantics.
4. Publish compatible Ely and gpui-mobile revisions and update downstream pins when release is requested. Register the Ely owner reporter in each mobile host before relying on it.
