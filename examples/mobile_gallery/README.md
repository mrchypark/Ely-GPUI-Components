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

Before porting, existing page cfgs were inspected. Shell status/system/windows modules and their exports now exclude iOS/Android; mobile shell rendering keeps columns and bars with an explicit desktop-feature limitation. Existing terminal cfgs/replay, tooling debug/release cfgs, wasm paths, and theme window behavior were retained. Theme's frosted-window action still depends on host window support and logs an open failure.

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
- Run all six included tests: catalog uniqueness, bundled asset round trips, all-44-page rendering, wheel scrolling, fixture removal/recreation, and original-panic preservation when cleanup fails. The all-page test draws at 390×844 and checks nonempty page bounds, visits and invalid-slug rejection. The wheel test checks positive range, real content movement, fixed headers, persistence and navigation reset through the retained ScrollHandle. Header debug selectors are independent of the cleared specimen-probe map. This does not test native touch synthesis.
- The test-scope RAII fixture verifies extracted bytes and removes its exclusively created directory before releasing the lock. Normal cleanup asserts removal and absence; cleanup failure during panic unwinding reports the owned path/error without replacing the original panic. Pre-existing directories are refused. These changes supersede the earlier fixture lifecycle; the new task-workspace freeze is `outputs/gallery-scroll-cleanup-frozen-20261018.sha256`. The six tests are unrun for this revision.
- Check the upstream desktop gallery and wasm gallery to catch menu/resource regressions. Confirm desktop menu commands and context-menu buttons still work, document editor heights remain correct, and math renders with upstream's API.
- Build each intended native target through its owner-managed adapter. Exercise all 44 pages, offline image decoding, document/chat media, Light/Dark with no system menus, navigation/scrolling, keyboard and safe-area updates. Platform-specific specimens and asynchronous images require runtime evidence; the host test does not establish their visual acceptance.
