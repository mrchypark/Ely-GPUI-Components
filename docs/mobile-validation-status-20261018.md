# Mobile publication validation status — 2026-10-18

This records the existing app Git GPUI working tree for publication on
`feature/mobile-ui`, against `main` at
`083ef856968812ef83f6dc2db0cc191c50b83cdc`. No development, dependency changes,
builds, or tests were performed during publication preparation.

## Source identity

Pre-commit HEAD: `f98d0005e1805405cf8f71d758e57714a6bf9a25`.
The initial inventory contains 164 modified tracked files and 38 untracked
Rust source/test files, with an empty staging area. Existing tests are included
as source; none were newly authored or executed for publication.

- Final validated source inventory SHA256:
  `3e2e76ae70ff82879cdfc236993d3aff786b7bab2181db0534a7822a1e481b86`.
  Its file hashes still match the working tree during this audit.
- Tracked dirty binary-patch SHA256:
  `2eeec4a67570794fc97381dc3a96c312acaacf9d6c162b9186cd6af59d8dfad8`.
- Publication audit's sorted 202-path dirty-file SHA256 manifest hash:
  `6cd48b31fdca31e1c0b646b28ca677786a7a95ab19d09af18fdd7200176ad501`.
  This covers the initial dirty files, including untracked source, before this
  status document was added. It is distinct from the complete source inventory.

Exact paths, hashes, initial staged/unstaged patches, and publication exclusions
are retained outside the repository in the task workspace's
`outputs/ely-app-publication-audit/`. Generated `target/` contents are excluded
and untouched. The modified `examples/mobile_gallery/README.md` is excluded
because it contains absolute workstation paths; it remains unchanged locally.

## Included API and dependency boundary

This publication includes the existing `forms::TextInputOwner`,
`TextInput::input_owner_observer`, IME owner reporting, focus/blur activation
handling, module registration/exports, and owner tests. The five-path exclusion
described in the historical clean publication-v1 reports applied only to that
candidate. It does not describe this native app publication. In particular,
the exclusion language and pending checks in `mobile-component-coverage.md`,
`mobile-support.md`, and older gallery documentation are historical; this
document supplies the current publication boundary. The old clean candidate's
1825-test PASS must not be represented as owner-enabled full-suite evidence.

The app retains Git GPUI revision
`1a28cff4b409169bac058bca40dfbfeb7621d19b`; it is not the upstream registry
line. Cargo.toml SHA256 is
`609589dbb8b26d7cd4e5e9cb7a9d8316fc70c3ee25bd211b0d939049819b3d25`,
and Cargo.lock SHA256 is
`090442505a4ae17a7942faa19a7e9a5a4370eb1980f4d6ee0dea1c59bf0fc6d7`.
Both are unchanged by this publication preparation. Existing branch history
already limits gpui_platform to non-iOS/non-Android targets.

The original GPUI integration is a separate sibling checkout. Its Ely dependency
is the local path `../../../../../ely-mobile-mrchypark/work/ely`, with default
features disabled; it is not a published or portable registry dependency.
Its GPUI revision remains the same Git revision, and the recorded integration
wgpu identity is 29.0.4. Integration artifacts and backend changes are not part
of this commit.

## Existing validation evidence

Evidence comes from the task-local
`outputs/resumed-validation-20261018/final-closeout/README.md` and
`outputs/final-bounded-coverage-ledger-20261018.md`. These external evidence
files and binaries are not included in the repository.

- App gallery: 38 distinct passing cases, as an incremental union of 34
  unaffected r3 cases, three repaired r4 cases, and one r5 Forms allocation
  case. This is not a fresh 38-test r5 run. The r5 assertion covers actual
  field allocation at 320/390 widths.
- App affected document modes: four PASS. Earlier hash-attributed responsive
  and r3 component checks remain recorded in the ledger.
- Earlier desktop checks passed; no fresh r5 standalone desktop check is claimed.
- Native gallery: r5 build/link/sign/strict verification passed. Owner-reported
  runtime evidence covers a bounded nine-group union, with explicit native
  320-width, trailing-content, and split-mode limitations.
- Original sibling integration: host selfchecks two PASS, locked/offline
  target metadata three PASS, iOS package/link/sign checks PASS and bounded
  input/menu/canvas runtime PASS reported by its owner.
- Original Android integration: Rust build, Gradle package, signature, manifest,
  and DEX checks passed for APK SHA256
  `ded6b1e975a77a3c08a5a088c2bc4cfc7f0fb1df7338cc79531cd627d6e92ec3`.
  The owner reported matching installed APK, both startup SELFCHECK markers,
  `NATIVE_INITIALIZED=true`, live process and resumed activity. The launch wait
  itself timed out; later observations support only a narrow startup smoke.

**Full Android UI/runtime acceptance: NOT VALIDATED.** Emulator UI access was
blocked. Startup does not prove frame presentation, menu/input interaction,
physical-device behavior, GPS/BLE, or complete IME. Packaged
BLUETOOTH_SCAN/CONNECT permissions were absent. No broad native acceptance,
all-624-story interaction, full 44-page behavior, glyph correctness, or complete
editor-mode reachability is claimed. Historical failures and invalid artifacts
remain historical, rather than being relabeled aggregate PASS.

The Android owner subsequently confirmed app, qemu, and netsim absence and an
empty adb device inventory. This supersedes earlier cleanup-pending entries;
it is owner-reported evidence, not a new runtime check by this publication task.
No generated resource was cleaned or modified during publication preparation.
