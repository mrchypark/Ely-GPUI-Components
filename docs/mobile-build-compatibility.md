# Mobile compatibility: two independent branches

2026-10-15. This forward port is an upstream submission candidate, not a migration of the current application or gpui-mobile backend. All new-source checks below apply to source commit `d1b64ab6bd793a1fa2c26e8d690aebaaf205aa87`; a later documentation-only head is separate.

## Dependency identity

| Contract | Existing application compatibility line | Upstream submission candidate |
| --- | --- | --- |
| Branch | `feature/mobile-ui` | `feature/mobile-ui-upstream` |
| Revision | `7f6a3f9` | Source `d1b64ab6bd793a1fa2c26e8d690aebaaf205aa87`, based on `5a53c08c844210ea80c6a92aec708e2de8ffa721` |
| Ely original base | `083ef856968812ef83f6dc2db0cc191c50b83cdc` | Author Gitea main at the base above |
| GPUI package | `gpui` 0.2.2 | `gpui-pre` =0.3.8, imported as `gpui` |
| GPUI source | `git+https://github.com/zed-industries/zed`, commit `1a28cff4b409169bac058bca40dfbfeb7621d19b` | crates.io registry; package metadata declares Zed `279fe070bb389b79652e52065b2f001edcc0b11b` |
| Platform | Git `gpui_platform` at the same Zed revision | registry `gpui-pre-platform` =0.3.8, imported as `gpui_platform` |
| wgpu lock entry | registry 29.0.4 | registry 29.0.4 |
| Lockfile policy | Preserve the existing pinned line | Preserve upstream Cargo.lock; do not copy the historical lock |

The independent reviewer verified `package.metadata.gpui-pre.zed-rev = "279fe070bb389b79652e52065b2f001edcc0b11b"` in the official gpui-pre and gpui-pre-platform 0.3.8 Cargo.toml files. Neither package provides `.cargo_vcs_info`. This establishes declared metadata provenance, not byte-equivalence to that Zed source tree. The gpui-pre archive checksum matches the lock checksum below. Both branches have the same registry wgpu 29.0.4 identity and checksum; their GPUI package identities remain different and their types are not interchangeable.

Final source review reported no functional findings and no required GPUI API adaptation. Differences from the pinned port in `src/layout/mod.rs` and `src/layout/scroll.rs` preserve existing upstream narrow-layout and ScrollArea features. Registry dependency aliases remain intact. Native `PinchEvent` retains the incremental `1.0 + delta` contract. The current raw-touch recognizer still ignores extra contacts and does not recognize pinch; a host must supply the raw-contact route or recognized pinch explicitly. Native pinch dispatch uses hit-testing and does not establish pointer capture.

The upstream Cargo.lock remains unchanged with SHA-256 `8e1e1d4091257d7ed4153d7da826cdf08405505ba2b70f0b84d9ffd6dfcc571f`, as reported by the source/build coordination. This lockfile hash is distinct from the package archive checksums below.

The [adapter contract](mobile-support.md) distinguishes second-finger preview cancellation, which preserves internal pinch state, from external cancellation, which resets contacts. It also restricts keyboard-rectangle conversion to a full-width bottom dock. These are integration requirements, not claims of an implemented raw OS adapter or device runtime support.

Observed upstream lock checksums:

- gpui-pre: `d7f0bc3ebba070f8981b3f8e905a12f47d27794a30d029e59cdb9edf7a4b3360`
- gpui-pre-platform: `e200b7b78836f1a0fc70767d971768e88f29d8b91b697285c15fb8e373635a3e`
- wgpu: `76e8840e1ba2881d4cbb18d2147627a56af426ff064c0401eb0c8410c6325d07`

Ely core has no mobile backend dependency. The integration application combines Ely with a compatible platform implementation; neither library core depends on the other. Any future application transition requires coordinated dependency resolution and renewed compile, link and runtime checks.

## Validation matrix

Historical evidence below is reported by the pinned branch's preserved logs. None of those passes validates this forward port. New results were confirmed against the source revision above. Original logs are in the task's `work/upstream-logs/`; deliverable logs are designated for `outputs/upstream-validation/`.

| Check | Pinned branch evidence | New upstream branch evidence |
| --- | --- | --- |
| macOS all examples | Recorded exit 0 | Exit 0, 2m32s; `check-desktop-examples.log`; no Ely warnings, upstream block future-incompatibility warning only |
| iOS library and test compilation | Recorded exit 0; tests not executed | Exit 0, 3m16s, test-support; `check-ios-tests-online.log`; tests not executed |
| Android library type check | Recorded exit 0 | Exit 0, 1m19s, NDK 28.2/API 21; `check-android-lib-ndk.log`; configured-NDK retry after missing compiler |
| Shared showcase iOS test compilation | Recorded exit 0; no mobile launcher | Exit 0, 27.37s; `check-showcase-ios-tests.log`; tests not executed |
| Historical patch-map iOS type check | Recorded exit 0, without mobile backend | Not established for registry GPUI |
| Pure canvas tests | 12 standalone tests recorded passing | Exit 0; 12 standalone tests executed/passed; `pure-canvas-upstream.log` |
| iOS Clippy | Recorded exit 0 | Not performed |
| macOS library Clippy | Not recorded here | Exit 0, 16.47s, -D warnings; `clippy-desktop-lib.log` |
| Formatting / diff whitespace | Recorded exit 0 | Both exit 0; `fmt-final.log`, diff result confirmed separately |
| Target dependency trees | Recorded macOS/iOS checks | `tree-final-ios-gpui.log`: one gpui-pre 0.3.8; `tree-final-ios-wgpu.log`: empty, wgpu inactive; `tree-final-desktop-platform.log`: desktop platform present |
| Lockfile inspection | Git GPUI and registry wgpu observed | Registry packages/checksums observed; not a resolved target-tree check |
| Windows/Linux builds | Not established | Not established |
| Mobile linking, simulator/device execution, IME, visuals, FPS | Not established by Ely checks | Not established |

Record each new result with source revision or diff identity, target, command, exit status and log path. Keep compile-only, executed tests, linking and runtime evidence separate. A lock entry does not prove that package is active for a target. The older combined-backend probe's 12 platform API failures cannot be considered resolved by an Ely-only check.

## Contribution route

Canonical upstream: https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components.
The authorized own repository is https://github.com/mrchypark/Ely-GPUI-Components.
CONTRIBUTING.md requires Gitea AGit submission to `refs/for/main`; ordinary GitHub publication is not upstream submission. No AGit push, upstream issue, PR or merge is authorized in this task. A local PR draft is preparation only. Upstream also requests discussion before large changes, repository checks, and light/dark screenshots for visual changes; these remain submission prerequisites where applicable.
