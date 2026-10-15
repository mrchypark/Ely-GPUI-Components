# Mobile build compatibility

## Source contract (2026-10-15)

- Ely base: `083ef856968812ef83f6dc2db0cc191c50b83cdc`.
- GPUI source: `https://github.com/zed-industries/zed`, revision
  `1a28cff4b409169bac058bca40dfbfeb7621d19b` (package version `0.2.2`).
- wgpu: crates.io registry `29.0.4`, as recorded in the unchanged Cargo.lock.
- Ely remains a UI library. An application supplies the mobile GPUI platform;
  neither Ely nor gpui-mobile core should depend on the other.

The original GitHub upstream returned 404. The author's live canonical source is
[Gitea](https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components).
Its API identifies it as neither fork nor mirror; current README and Cargo.toml
both point to Gitea. Its main was `5a53c08c844210ea80c6a92aec708e2de8ffa721`,
which uses `gpui-pre =0.3.8`. This branch deliberately starts at the agreed
historical Ely base. Rebasing onto current main requires a coordinated GPUI
transition across the application, Ely and gpui-mobile, plus renewed compile,
link and runtime verification; do not mix the two GPUI sources.

[mrchypark's repository](https://github.com/mrchypark/Ely-GPUI-Components) is an
independent history-preserving fork, not a native GitHub fork. The verified
Gitea clone retains `upstream`; `origin` uses GitHub SSH. Initial main and
feature/mobile-ui refs were both verified at the historical Ely base.
HTTPS push lacked workflow scope; existing SSH authentication successfully
published the intact source history, including upstream workflows.

CONTRIBUTING.md asks contributors to discuss large changes in an issue first,
run the repository checks, and provide screenshots for visual changes. No issue
or upstream PR was published by this work. Keep this manifest fix independently
reviewable; the historical branch is not a claim of compatibility with main.

## Manifest behavior

`gpui_platform` is excluded only on iOS and Android. Its existing default
`runtime_shaders` forwarding remains enabled on targets where that dependency
exists, including macOS. Desktop examples retain the platform and font-kit.
Web retains gpui_platform because the gallery calls `web_init`.
The mobile library does not compile a desktop platform to obtain shader flags.
Desktop example launchers are not mobile application entry points.

## Evidence and limits

Logs are outside the checkout in `../build-logs/`.

| Check | Result |
| --- | --- |
| Locked cargo tree, macOS/iOS/Android | Passed; mobile trees omit gpui_platform |
| macOS feature tree | font-kit and runtime_shaders remain enabled |
| iOS feature trees | Ely default runtime_shaders active; gpui_platform absent |
| macOS/iOS duplicate trees | No duplicate GPUI or wgpu entries |
| Cargo.lock inspection | One GPUI source at the contract revision; one registry wgpu 29.0.4; no lockfile change |
| All-target offline duplicate tree | Incomplete: uncached accesskit_atspi_common download required |
| Final iOS library and tests, test-support enabled | **Exit 0**, 30.54s, no warnings; `check-final-ios-tests.log`; tests compiled, not executed |
| Final shared showcase view iOS rlib | **Exit 0**, 13.81s; `check-final-showcase-ios.log`; no mobile launcher, link or execution |
| Final source-identical patch-map iOS library | **Exit 0**, 1.61s; `check-final-patchmap-ios.log`; no link or execution |
| Final desktop library and mobile_showcase check | **Exit 0**, 1m54s; `check-final-desktop.log`; upstream block 0.1.6 future-incompatibility warning only |
| Final Android library, NDK 28.2 / API 21 | **Exit 0**, 1m26s; `check-final-android.log`; includes final pinch handling |
| All desktop examples check | **Exit 0**, 5.27s; `check-final-examples.log`; compilation only |
| Standalone pure canvas tests | **12 passed**; `pure-canvas-tests.log`; standalone shim, not GPUI UI tests |
| iOS clippy library/tests with test-support and -D warnings | **Exit 0**, 41.81s; `clippy-final-ios.log` |
| cargo fmt --check and git diff --check | **Exit 0** for both |
| Link, simulator launch, hardware, FPS | Not verified by these checks |

The frozen source includes commits `141f3b4`, `870a1b5` and `1f215e9`, including
the finalized pinch handling. Cargo.lock is unchanged. Final iOS, Android and
desktop compilation passes. Only the 12 standalone pure tests were executed;
GPUI UI test execution and application runtime remain unverified. These results
do not claim a full-suite run.

The earlier patch-native feasibility probe reported 12 mobile platform API
errors after aligning this GPUI revision and wgpu source. Removing Ely's desktop
dependency does not establish that those backend errors are resolved.

## Reproducing the checks

Earlier checks encountered ENOSPC; the final iOS checks above succeeded after
recovering task-generated artifacts. Capacity history and intermediate failures
remain in the work logs. Reuse one target directory and profile, and run checks
sequentially. From the repository root:

```sh
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$PWD/target"
cargo +1.97.1 check --locked --lib --tests --features test-support --target aarch64-apple-ios-sim
cargo +1.97.1 check --locked --lib --example mobile_showcase --target aarch64-apple-darwin
cargo +1.97.1 check --locked --examples --target aarch64-apple-darwin
cargo +1.97.1 clippy --locked --lib --tests --features test-support --target aarch64-apple-ios-sim -- -D warnings
cargo fmt --check
git diff --check
```

For Android, install the Rust target and use an existing NDK. The recorded check
used NDK 28.2.13676358 and API 21 on macOS:

```sh
export CC_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android21-clang"
export AR_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
cargo +1.97.1 check --locked --lib --target aarch64-linux-android
```

Changing profile settings can invalidate cached artifacts. These commands check
Rust code; they do not execute tests or demonstrate a working platform backend.
The mobile_showcase example is registered explicitly in Cargo.toml.
Its desktop launcher is not a mobile application. A separate scratch package at
`../showcase-check` points its library target directly to the shared view source,
with Ely's path dependency and the same GPUI pin; check that rlib for iOS without
inventing a mobile main. This is type checking only, not linking or execution.

The external probes use the same environment and pins:

```sh
cargo +1.97.1 check --offline --locked --lib --target aarch64-apple-ios-sim --manifest-path ../showcase-check/Cargo.toml
cargo +1.97.1 check --offline --locked --lib --target aarch64-apple-ios-sim --manifest-path ../patch-map-check/Cargo.toml
```

Those scratch packages are verification fixtures outside this repository, not
shipped mobile applications. Standalone Rust shims are separate partial evidence
and must not be reported as tests passing against GPUI.
