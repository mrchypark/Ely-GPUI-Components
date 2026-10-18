# Mobile validation status — 2026-10-18

This publication captures the already-written registry upstream changes, including
the formatter repair and test fixtures. No development, build, test or formatting
was performed during publication preparation. Earlier pending entries in the
mobile support and component coverage documents describe historical snapshots;
the results below are the later bounded closeout, not whole-library acceptance.

## Dependency and source identity

The intended PR base is the fork-only `upstream/gpui-pre-0.3.8` branch at
`5a53c08c844210ea80c6a92aec708e2de8ffa721`. Fork `main` at `083ef85` is the
GitGPUI application line: using it would include 146 unrelated upstream commits.
This tree retains registry `gpui-pre`, `gpui-pre-platform` and `gpui-pre-web`
at exactly `0.3.8`; no application GitGPUI dependency is substituted.

Before publication, `feature/mobile-ui-upstream` was at
`e2011a74ddfa75845eea9b9c2dba27ed131c89da` with 168 modified tracked files and
37 untracked Rust files. All 205 existing changed files are included unchanged.
The three historical IME files (`src/forms/text/{mod.rs,ime.rs,ime_tests.rs}`)
are included in this current-source publication; they were excluded from the
earlier clean publication candidate. That older candidate's results cannot
certify these files or native IME behavior.

SHA-256 provenance:

| Inventory | SHA-256 |
| --- | --- |
| Prior live full inventory | `3fcfe895ba7638fc35396fd5b59ab2c65048bd3f5abfd20eba62f5d5d16fe790` |
| Prior immutable r5 inventory | `a971a39f515381b40bf621635e64aafae1dc74817ff0e06b4b8e1d761caf036b` |
| Prior/current tracked dirty patch | `fdd7e167c4f3755c8bf80c484836a3188920a54c00ea0bacd44395781e81c3b7` |
| Prior untracked inventory | `773d7e99089b4d9e0f86a75c7dc2469617f64ef734693e8e0666ed7022e8f39f` |
| Publication 205-file inventory | `f684eda1479c752b4fddfb1e3adf0cfd16aecb353f20cf384566d40642a4c624` |
| Cargo.toml | `fbdd0efde22c67afe1ddc28f0cece3b8caf1c12a2cb04fa53f92118902f481ad` |
| Cargo.lock | `8e1e1d4091257d7ed4153d7da826cdf08405505ba2b70f0b84d9ffd6dfcc571f` |

The prior live inventory was rechecked successfully before staging. Cargo files
are unchanged from the pre-publication HEAD. The branch already contains earlier
manifest changes relative to its upstream base, including mobile target gating.

## Existing upstream host evidence

| Scope | Recorded result |
| --- | --- |
| r5 gallery | 38/38 PASS, zero failures/ignored/filtered, 83.53 s; runner12144 |
| r5 formatter correctness | 29 PASS, including four differential/node-count regressions; runner96095 |
| r5 formatter timing | One strict 8× test PASS, one invocation; runner96095 |
| r5 document modes | 5 PASS; runner96095 |
| Math/diagram repair | Math 11 PASS; diagram 5 PASS, 1 intentionally ignored; runner8647 |
| Responsive library scopes | Forms154, lists45, tables76, navigation46, files54 PASS; earlier attributed runs |
| r3 menu/navigation/document/collaboration | 68/46/426/16 PASS; formatter timing excluded at that stage |
| Desktop gallery | Earlier runner53121 PASS; no fresh r5 standalone desktop check |

These are source-attributed incremental results, not one fresh full-library run.
The earlier clean candidate's 2237 PASS / 2 FAIL / 1 ignored remains historical.
Cached wrong-artifact run54840 remains INVALID; actual timing failure58440 is
preserved, with the later r5 strict test providing the repair evidence.

## Native acceptance and remaining work

**Upstream Android UI/runtime acceptance: UNVALIDATED.** The native gallery and
original integration smoke results use the separate application GitGPUI line.
They do not validate this registry dependency line.

For provenance only, the app-line Android owner reported the installed APK
`ded6b1e975a77a3c08a5a088c2bc4cfc7f0fb1df7338cc79531cd627d6e92ec3`, two startup
SELFCHECK passes, `NATIVE_INITIALIZED=true`, a live process and resumed activity.
The launch wait timed out; frame presentation and menu/input UI were not accepted.
The owner subsequently confirmed app/qemu/netsim absence and an empty adb device
inventory. Startup and cleanup success are separate from UI acceptance.

No claim is made for all 44 pages/624 stories, native 320-width behavior, complete
trailing-content or split-editor reachability, glyph/emoji correctness, complete
IME, rotation, multitouch, accessibility, lifecycle, performance or OS services.
Calendar, Dashboard, Settings, Account and Onboarding work remains incomplete;
hidden-editor and emoji design fixes are not included as completed features.
The coverage document's per-export S0–S7 criteria remain the acceptance scope.

## Retained evidence

The following task-local artifacts live outside this repository and are not
distributed with it:

- `outputs/resumed-validation-20261018/final-closeout/README.md` and its source,
  dirty-patch and untracked inventories.
- `outputs/final-bounded-coverage-ledger-20261018.md` (later evidence supersedes
  earlier pending statuses without rewriting historical results).
- `outputs/final-written-code-coverage-20261018/README.md` (earlier classification).
- `outputs/ely-upstream-publication-audit-20261018/` (exact publication inventory,
  SHA-256 checks and staging records).

See [component coverage](mobile-component-coverage.md) and
[mobile support](mobile-support.md) for the broader contract and remaining scope.
