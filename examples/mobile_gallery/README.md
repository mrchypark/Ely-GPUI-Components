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

Persistent Previous/Next/Catalog controls sit outside a two-axis specimen scroll area within usable bounds. Wide specimens remain horizontally scrollable, not universally redesigned. One page renders at a time. Mobile shell compilation excludes desktop status/system/window modules and displays the limitation while retaining bars/columns. Other platform-specific specimens still need simulator checks.

Selectors: gallery-page-{slug}, gallery-status, gallery-content. Navigation probes: gallery-previous, gallery-next, gallery-catalog. Existing specimen probes remain available. select(slug,cx) validates addresses, resets scroll and clears old probe bounds; current_slug()/visits() report navigation, not component interaction counts. Existing Page.script is input inventory, not a mobile runner for AppKit steps.

Tests: catalog address uniqueness; all-page entity selection plus unknown-address rejection using init_for_tests; bundled file uniqueness/nonempty/load round-trip. No Cargo or simulator run by this worker. rustfmt and diff checks passed. Native decoding/display and file extraction still need the parent-owned build/runtime checks; offline bytes being present does not itself establish every specimen's visual acceptance. Source ready for integration freeze.

## Host navigation failure correction

Initial host run: 2 PASS / 1 FAIL (`menus/bar.rs`: a menu bar needs menus). The empty mobile app_menus fixture was the cause. Shared gallery/menu_model.rs now supplies View → Light/Dark to desktop and mobile; mobile init registers the actual theme actions. The MenuBar specimen falls back to that model when the platform returns no menus or an empty menu list. It does not require calling OS set_menus. Desktop action handlers and menu contents are preserved. The all-page navigation test remains unchanged and must be rerun by the build owner. Source frozen after rustfmt/diff checks; no worker build. Original failure log retained.

## Explicit per-page frame coverage

Navigation test now sizes the window to 390×844 and, after each of all 44 selections, explicitly refreshes and draws the window, clears the GPUI element arena, then asserts that the current page-specific gallery-rendered-{slug} marker has nonzero layout bounds. Original slug/visit and unknown-address assertions remain. This is full page render/layout coverage, not every offscreen component interaction or asynchronous image decode completion.

A serialized test-only fixture uses one process-level OnceLock root and a test-scope RAII guard, which extracts/checks all bundled bytes and removes its private directory before releasing the lock. Normal cleanup explicitly asserts successful removal and absence. If cleanup fails during an existing panic, it reports the owned path/error without a second panic. Repeated runs reuse the resolver address and recreate bytes; pre-existing directories are refused and no native app directory is used. Cleanup regressions cover removal/recreation and preservation of the original panic when removal fails. Pinned TestAppContext constructs an empty AssetSource and exposes no public replacement API; tests retain init_for_tests. Extracted document/chat paths are available, while embedded-image/icon visual acceptance remains native evidence rather than a claim of this host test.

The wheel regression uses the persistent ScrollHandle at 390×844 to check positive range, actual catalog/navigation content movement, a fixed header, retained position on redraw, and reset on page transition. Header geometry uses frame debug selectors, independent of the cleared specimen-probe map. This is GPUI wheel/layout coverage, not native touch delivery assurance. The six current tests remain unrun for this revision; the new source freeze is `outputs/gallery-scroll-cleanup-frozen-20261018.sha256` in the task workspace.
