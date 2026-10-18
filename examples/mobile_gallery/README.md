# Shared mobile gallery — upstream integration

Use `lib.rs` as the crate root (`[lib].path`) of a parent-owned overlay manifest. It shares all 44 entries in `../gallery/pages/mod.rs`; pages and assets are not duplicated. This directory is not a standalone Cargo package or an automatically discovered executable example.

## Host contract

1. Use upstream Ely and its exact `gpui-pre = 0.3.8` registry identity for a registry host. A native adapter must supply the same GPUI identity as its Ely dependency. Do not copy the source branch's git pin into a registry overlay or change existing native pins. Overlay dependencies are `gpui`, `ely-gpui-component`, `anyhow`, `image`, `jiff`, `log`, `serde` (derive), `serde_json` (preserve_order), `web-time`, and `qrcode` (the terminal replay fixture). Tests require Ely/GPUI `test-support`.
2. Register `assets::GalleryAssets` with the application.
3. Call `assets::prepare_files(&app_private_asset_directory)?` before rendering. The host obtains a writable app-private directory at runtime, propagates extraction errors, and removes that directory only after the gallery stops using it.
4. Initialize Ely, then call `init(&mut App)` to register theme actions, touch mode, keybindings and i18n. Mount `MobileGallery::new(window, cx)`.
5. Feed native safe-area and keyboard obstruction through `set_insets(ViewportInsets, Pixels, &mut Context<Self>)`.

The existing 122 gallery files are embedded by `embedded.rs`. `gallery-assets/` keys use `Resource::Embedded`; legacy document/chat string APIs use extracted files through `assets::file`. The desktop/web helper is identity, preserving existing filesystem paths and URLs. Ely fonts, icons and the gallery mark still use the existing asset source.

Previous/Next/Catalog controls remain outside the two-axis specimen scroll area. `select(slug, cx)` validates slugs, clears old probes and resets scroll; `current_slug()` and `visits()` expose navigation state. Selectors include `gallery-page-{slug}`, `gallery-rendered-{slug}`, `gallery-status`, and `gallery-content`. Existing `Page.script` data remains the desktop interaction inventory, not a mobile input runner.

The shared View → Light/Dark menu works without a platform menu bar. Desktop action handlers remain unchanged; a missing or empty platform menu list falls back to the shared model.

## Port audit and source freeze

Ported from the working tree based on source commit `27fcea58d5ea0a8adb5994f4cdef551a0bf94e56` into upstream base `6b6369a3e06543d4d12224680e36656875a9afe0`. The commit alone is not the source freeze: the task's `outputs/upstream-gallery-integration-20261018/source.sha256` records the working files, excluding terminal paths owned by another worker. Source files were hash-verified after integration.

Before porting, existing page cfgs were inspected. Shell status/system/windows modules and their exports now exclude iOS/Android; mobile shell rendering now retains bars, adds real inline specimens, and preserves exact capability sections; TitleBar and ColumnShell now use safe touch headers with meaningful local actions. Existing terminal cfgs/replay, tooling debug/release cfgs, wasm paths, and theme window behavior were retained. Theme's frosted-window action still depends on host window support and logs an open failure.

Upstream's two-argument `Latex::parse`, context-menu button row, document editor heights, and Gitea URLs were preserved. Registry GPUI menu and test/window API signatures were inspected locally. No `src`, manifests, lockfiles, terminal files or native pins were edited by this port.

Exact upstream files added:

- `examples/mobile_gallery/lib.rs`
- `examples/mobile_gallery/view.rs`
- `examples/mobile_gallery/assets.rs`
- `examples/mobile_gallery/embedded.rs`
- `examples/mobile_gallery/README.md`
- `examples/gallery/menu_model.rs`

Exact upstream files modified:

- `examples/gallery/main.rs`
- `examples/gallery/assets.rs`
- `examples/gallery/ui.rs`
- `examples/gallery/pages/mod.rs`
- `examples/gallery/pages/shell/mod.rs`
- `examples/gallery/pages/menus/more.rs`
- `examples/gallery/pages/chat/blocks.rs`
- `examples/gallery/pages/chat/compose.rs`
- `examples/gallery/pages/documents/editing.rs`
- `examples/gallery/pages/documents/reading.rs`

## Verification handoff

No Cargo, build, simulator or runtime test was run during this port while Darwin owned compilation. Scoped rustfmt, diff checks, catalog/asset inventory checks and source checksums are the static checks; they do not prove typechecking or native rendering.

After the build owner releases the build lane:

- Create/use a parent-owned upstream overlay targeting this `lib.rs` and upstream Ely with a consistent registry GPUI identity. Existing native overlays still target the source branch and were not modified.
- Run all 26 current gallery tests, including catalog uniqueness, bundled asset round trips, all-44-page rendering, wheel scrolling, fixture removal/recreation, and original-panic preservation when cleanup fails. The all-page test draws at 390×844 and checks nonempty page bounds, visits and invalid-slug rejection. The wheel test checks positive range, real content movement, fixed headers, persistence and navigation reset through the retained ScrollHandle. Header debug selectors are independent of the cleared specimen-probe map. This does not test native touch synthesis.
- The test-scope RAII fixture verifies extracted bytes and removes its exclusively created directory before releasing the lock. Normal cleanup asserts removal and absence; cleanup failure during panic unwinding reports the owned path/error without replacing the original panic. Pre-existing directories are refused. These changes supersede the earlier fixture lifecycle; the new task-workspace freeze is `outputs/gallery-scroll-cleanup-frozen-20261018.sha256`. The current 26-test gallery suite requires a coordinated rerun for this source batch.
- Check the upstream desktop gallery and wasm gallery to catch menu/resource regressions. Confirm desktop menu commands and context-menu buttons still work, document editor heights remain correct, and math renders with upstream's API.
- Build each intended native target through its owner-managed adapter. Exercise all 44 pages, offline image decoding, document/chat media, Light/Dark with no system menus, navigation/scrolling, keyboard and safe-area updates. Platform-specific specimens and asynchronous images require runtime evidence; the host test does not establish their visual acceptance.

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
CARGO_TARGET_DIR="<gpui-mobile-checkout>/example/target" cargo +1.97.1 test --locked --offline --manifest-path outputs/mobile-gallery-simulator-20261017/Cargo.toml -p ely-mobile-gallery --features test-support --lib -j1 -- --test-threads=1 --nocapture
CARGO_TARGET_DIR="<ely-upstream-checkout>/target" cargo +1.97.1 test --locked --offline --manifest-path outputs/upstream-mobile-gallery-host-20261018/Cargo.toml -p ely-upstream-mobile-gallery-host --features test-support,runtime_shaders --lib -j1 -- --test-threads=1 --nocapture
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
