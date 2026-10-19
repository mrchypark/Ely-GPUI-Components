# Shared mobile gallery — frozen integration contract

Use `lib.rs` as the crate-root `[lib].path` in the parent-owned task-local overlay manifest, not as a nested module. All **44** current catalog entries reuse the original page modules; none are copied.

## Native initialization

1. Keep the app-compatible GPUI pin and local Ely path. Overlay dependencies need gallery versions/features of gpui, ely-gpui-component, anyhow, image, jiff, log, serde, serde_json and web-time. Tests need Ely/GPUI test-support.
2. Register this crate's `assets::GalleryAssets` with the application.
3. Call `assets::prepare_files(&app_private_asset_directory)?` once before rendering. Pass an app-private writable subdirectory obtained at runtime (iOS sandbox/Android files directory). No random install path or host path is compiled in. Failure must propagate and stop gallery launch. Extraction writes only the named gallery files; the adapter owns cleanup when finished. Do not remove while the gallery is running.
4. Initialize Ely, then call this crate's `init(&mut App)` (keybindings, i18n, touch_mode=true). Create `cx.new(|cx| MobileGallery::new(window, cx))`.
5. Feed native safe area and keyboard obstruction via `set_insets(ViewportInsets, Pixels, &mut Context<Self>)`.

`ELY_GALLERY_ASSETS` is no longer used. `asset!` produces `gallery-assets/<name>` keys. The 122 existing files (~2.8MiB on disk) are bundled with include_bytes via embedded.rs, with no new dependency or duplicate asset files. GalleryAssets serves them through GPUI Resource::Embedded and delegates Ely fonts/icons/custom mark to the existing source. Gallery picture/resource helpers and direct GPUI string sources use embedded keys. Legacy document/chat string media APIs require physical paths: those example callsites use assets::file and the extracted runtime directory. Desktop/web assets::file is identity, preserving their original paths/URLs. Runtime materialization avoids modifying product media APIs or GPUI.

## Navigation and verification

Persistent Previous/Next/Catalog controls sit outside a two-axis specimen scroll area within usable bounds. Wide specimens remain horizontally scrollable, not universally redesigned. One page renders at a time. Mobile shell compilation excludes desktop status/system/window modules and displays the limitation while retaining bars and providing the mobile inline specimens described below. Other platform-specific specimens still need simulator checks.

Selectors: gallery-page-{slug}, gallery-status, gallery-content. Navigation probes: gallery-previous, gallery-next, gallery-catalog. Existing specimen probes remain available. select(slug,cx) validates addresses, resets scroll and clears old probe bounds; current_slug()/visits() report navigation, not component interaction counts. Existing Page.script is input inventory, not a mobile runner for AppKit steps.

Tests: catalog address uniqueness; all-page entity selection plus unknown-address rejection using init_for_tests; bundled file uniqueness/nonempty/load round-trip. No Cargo or simulator run by this worker. rustfmt and diff checks passed. Native decoding/display and file extraction still need the parent-owned build/runtime checks; offline bytes being present does not itself establish every specimen's visual acceptance. Source ready for integration freeze.

## Host navigation failure correction

Initial host run: 2 PASS / 1 FAIL (`menus/bar.rs`: a menu bar needs menus). The empty mobile app_menus fixture was the cause. Shared gallery/menu_model.rs now supplies View → Light/Dark to desktop and mobile; mobile init registers the actual theme actions. The MenuBar specimen falls back to that model when the platform returns no menus or an empty menu list. It does not require calling OS set_menus. Desktop action handlers and menu contents are preserved. The all-page navigation test remains unchanged and must be rerun by the build owner. Source frozen after rustfmt/diff checks; no worker build. Original failure log retained.

## Explicit per-page frame coverage

Navigation test now sizes the window to 390×844 and, after each of all 44 selections, explicitly refreshes and draws the window, clears the GPUI element arena, then asserts that the current page-specific gallery-rendered-{slug} marker has nonzero layout bounds. Original slug/visit and unknown-address assertions remain. This is full page render/layout coverage, not every offscreen component interaction or asynchronous image decode completion.

A serialized test-only fixture uses one process-level OnceLock root and a test-scope RAII guard, which extracts/checks all bundled bytes and removes its private directory before releasing the lock. Normal cleanup explicitly asserts successful removal and absence. If cleanup fails during an existing panic, it reports the owned path/error without a second panic. Repeated runs reuse the resolver address and recreate bytes; pre-existing directories are refused and no native app directory is used. Cleanup regressions cover removal/recreation and preservation of the original panic when removal fails. Pinned TestAppContext constructs an empty AssetSource and exposes no public replacement API; tests retain init_for_tests. Extracted document/chat paths are available, while embedded-image/icon visual acceptance remains native evidence rather than a claim of this host test.

The wheel regression uses the persistent ScrollHandle at 390×844 to check positive range, actual catalog/navigation content movement, a fixed header, retained position on redraw, and reset on page transition. Header geometry uses frame debug selectors, independent of the cleared specimen-probe map. This is GPUI wheel/layout coverage, not native touch delivery assurance. The then-current seven-test snapshot required a coordinated rerun after the per-window bounds integration; the current suite count is 26 as detailed below; its freeze is `outputs/gallery-overlay-bounds-frozen-20261018.sha256` in the task workspace.

## Per-window overlay bounds

Every page render accepts an optional logical window area. Mobile supplies the area computed from its current safe insets and keyboard occlusion; desktop supplies None. Affected leaf fixtures and compound builders forward that value, including empty areas. Menu models use a stateless helper because they are not GPUI elements. MenuBar now receives usable_bounds; the preserved MenuBar regression checks inset containment, shrink, no-room closure and shared menu action dispatch.

The additional host regression activates the actual Share popover and Calendar EventPopover at 390×844 through the existing story selector. It checks nonzero inset containment, open-panel shrink to 226×150, zero-area closure, and explicit reopening after space returns. It keeps geometry on the MobileGallery instance and makes no claim about native input or inset delivery. All previous all-page, wheel, and cleanup regressions remain. Cargo and desktop compilation are pending the batch grant.

## Native specimen addresses (2026-10-18)

Page selection still opens the complete page. The visible `Story ›` button opens its first story; `‹ Story` / `Story ›` then select adjacent stories within that page, with disabled endpoints. `All` restores the full page. The label shows the story index, total, and title. Previous/Next page and Catalog remain available. Catalog temporarily hides the specimen without discarding the selection. The full-page scroll viewport remains available; story selection is not evidence that native scrolling works.

The catalog is the existing `../gallery/stories.json`: 44 pages, 624 story addresses. `select_story(page_slug, story_slug_or_exact_title, cx)` validates before changing state, resets scroll/probes, and selects the actual existing section without invoking its controls. `current_story_slug()` returns the selected address, or None for a full page. Selection and usable geometry stay on MobileGallery. Each selected render installs `ui::Story`, renders the page, removes the global, and checks `Story::missing`; unmatched or ambiguous catalog entries display the diagnostic instead of silently claiming coverage. Story routing needs no shared ui/probe changes. The mobile Shell leaf now resolves all 24 exact sections: 14 real component stories and 10 explicit capability sections. TitleBar and ColumnShell now use the frozen touch header with local Inbox/Archive and message actions; compact multi-column navigation remains a separate gap. Capability sections are not actual-component passes; they retain the full 624-address denominator.

Stable controls/probes: `gallery-story-previous`, `gallery-story-next`, `gallery-story-all`. Frame selectors also include `gallery-story-label`, `gallery-story-body`, and `gallery-story-error` (only on failure). Existing page/catalog selectors and specimen probes are retained. These are GPUI debug selectors/probes, not a claim that native accessibility exposes them to CUA.

Pending coordinated validation: **26 shared-gallery tests per repository** = original 10 (including four terminal replay tests), two existing Popover/EventPopover and MenuBar geometry regressions, three story tests,  eight mobile Shell tests, and three gallery caller regressions. Story tests:

- `view::story_tests::story_catalog_addresses_are_unique_and_cover_every_page`
- `view::story_tests::every_catalog_story_draws_with_identity_and_reveals_from_the_top`: selects all 624 addresses, explicitly draws twice, checks unique section matching, visible body origin, scroll reset, and removal of Story after each frame.
- `view::story_tests::story_buttons_preserve_page_catalog_and_reject_unknown_addresses`: button activation, endpoint behavior, overview/catalog/page transitions, and invalid-address rejection.

All prior geometry assertions are preserved; those tests now use per-view select_story instead of a persistent Story global. Host rendering does not establish native touch scrolling, every nested control interaction, embedded image decoding, or keyboard/IME behavior. Parent CUA owns native validation. The separately frozen adapter has four tests total (one existing plus three new keyboard/buffering/owner tests); they are not part of either 26-test gallery harness.

No Cargo or native run was made for this change. After all owners freeze, run the complete app and upstream host gallery suites serially with Rust 1.97.1, test-support, -j1, debug=0, incremental=0 and their existing caches. Upstream also uses runtime_shaders. Execute the tests (not only --no-run), retain logs and binary/source hashes, and verify automatic fixture cleanup plus any owned terminal children. Follow with adapter tests and the granted native build; parent performs actual specimen taps and records platform-unavailable entries separately.

Run from the task workspace root after the parent's compile grant; capture each command's output/exit code in a fresh provenance log and stop on failure:

```sh
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_TARGET_DIR=/Users/cypark/Documents/Codex/2026-10-15/gpui-mobile-mrchypark/work/gpui-mobile/example/target cargo +1.97.1 test --locked --offline --manifest-path outputs/mobile-gallery-simulator-20261017/Cargo.toml -p ely-mobile-gallery --features test-support --lib -j1 -- --test-threads=1 --nocapture
CARGO_TARGET_DIR=/Users/cypark/Documents/Codex/2026-10-15/ely-mobile-mrchypark/work/ely-upstream/target cargo +1.97.1 test --locked --offline --manifest-path outputs/upstream-mobile-gallery-host-20261018/Cargo.toml -p ely-upstream-mobile-gallery-host --features test-support,runtime_shaders --lib -j1 -- --test-threads=1 --nocapture
```

### Mobile Shell repair gate

The eight `pages::shell::mobile_tests` tests directly render the mobile Shell implementation on the host: all 24 exact sections (including 10 explicit capability notices), real update-banner actions, inline update-dialog dismissal, splash/About/connectivity state, local crash-report actions, MenuBar action dispatch and zoom, and pointer taps on visible update/crash/zoom controls. Keyboard checks remain separate. No assertion or test was removed by the compile repair. Pointer targets are measured from selectors on the actual hit elements; full component/viewport containment is required before dispatching each click, including after zoom.

WindowSwitcher retains its real registered-window contract; this gallery does not replace it with fake window entries. Inline SearchPalette is separate from the OS-popup QuickLauncher. The capability notices describe unsupported behavior and product gaps, not required testing hooks. The 26-test runtime suites remain pending after the compile repair; formatting checks do not establish a runtime pass.

### Real touch-header specimen integration

TitleBar and ColumnShell/ShellColumn keep their exact catalog addresses and use the existing builders under Theme.touch_mode. The local message example has real Inbox/Archive switching, message details, and archive/restore callbacks; it sends no mail and supplies no native-window action. ColumnShell currently demonstrates one phone-sized column; this does not certify compact multi-column navigation. Desktop fixtures and the ten remaining OS-capability sections are unchanged.

The eighth mobile Shell regression exercises both real headers at 390px and 320px, measures actual leading/action/body targets, requires 48px dimensions and full containment, checks state changes and absence of desktop controls/close requests. Existing Zoom pointer assertions additionally require measured minus/reset/plus targets to meet 48px without overlap at the encountered 100%, 90% and 110% scales. There are now 26 expected gallery tests, with all 24 Shell and 624 total addresses retained. New tests are authored but not executed; this integration does not claim native scroll or OS-window acceptance.

### Bounded palettes and reachable FileGrid callers

Navigation forwards the current optional usable rectangle to CommandPalette, SearchPalette, QuickOpen, and QuickSwitcher. Editor forwards it to GoToSymbol and ProjectSwitcher, Git to BranchSelector, and Terminal to CommandHistory. Shell forwards it to the existing real WindowSwitcher without changing managed-window registration or focus contracts. These callers use each component's usable_bounds builder on every render; desktop callers retain None and existing callbacks are unchanged. Runtime verification of the newly forwarded bounds remains pending.

On iOS/Android or in touch mode, the reachable launcher entry opens the existing inline SearchPalette, using the original search function and went/result callback. Desktop non-touch retains QuickLauncher::open. Inline search is labeled as such and is not a native popup-window support claim. Shell's QuickLauncher explanation is consistent with this distinction.

FileGrid uses available parent width in touch mode, retaining its 640px desktop wrapper. Mobile copy describes single selection followed by the real Open button; the original log callback also records a local result for visible verification, without opening files. The two added caller regressions exercise actual MobileGallery story selection: launch/search/result/Close with live safe-area/keyboard bounds and no new window, and FileGrid selection/Open with the actual file-grid-open hit target contained at 390px/320px. Runtime acceptance remains pending; total expected suite count is 26, adapter tests remain separate, and all 624 addresses remain in scope.
The third caller regression opens all six newly bounded wrappers through their actual gallery buttons, reveals offscreen openers with host wheel events, requires visible hit bounds, checks palette containment before and after a live keyboard-inset change, and taps the measured 48px Close control. It does not establish native touch scrolling. The expected suite total is 26; execution remains pending the coordinated verifier.
