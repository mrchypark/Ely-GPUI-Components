# Ely mobile and tablet gallery audit — 2026-11-12

This audit uses the [component inventory](mobile-component-coverage.md) and the
44-page, 624-story gallery catalog. A gallery story may contain several
components, and drawing it does not establish native interaction coverage.

| Check | Scope |
| --- | --- |
| Page resize | All 44 pages at 320×640, 640×280, 768×1024 and 1024×768 logical pixels. Change the live window size without selecting the page again; check that it draws, its scroll viewport fits, and page selection and visit count persist. |
| Story render | Draw all 624 catalog stories and check their identity at the gallery's standard phone test size. |
| State and touch | Focused host tests exercise representative forms, menus, navigation, lists, table editing, media scrubbing, and overlay behavior. |
| Native samples | Android API 34 emulator and iPad Pro iOS 26.2 simulator. Run the same menu action through native taps and change the viewport while the page remains mounted. These samples do not cover every story. |

All 39 gallery tests and 178 touch-filtered library tests passed. The outline
and scrubber checks also passed when run directly. On both native samples,
`New file` updated the caption to `ran new file`, and the caption persisted
after resizing. After removing shortcut hints from touch menus, all 60 menu
tests and all 39 gallery tests passed again.

On the final Android build at 720×1600 physical pixels (420 dpi), the
File menu showed the full `Save as…` label and tapping it produced
`ran save as`. At 900×1600, off-glyph taps on the expanded tree targets
collapsed and reopened Documents and checked Write. A tap outside the
Outline label text selected Quick start.

On the final iPad build, native taps selected a Tree row and collapsed its
children. Checking Write in CheckboxTree changed the caption from
`Granted: read` to `Granted: read, write`. The Outline selection marker
moved from Installation to Components.

Changes: the gallery catalog uses one column on phones, two from 600 logical
pixels and three from 900. In a short window the secondary gallery controls
scroll with the content. Touch menu bars and navigation triggers wrap at the
available width. Tree expansion and check controls and outline rows use finger
sized targets; the per-level tree indent remains compact. The seekable media
scrubber shows its knob without hover, and the media transport can wrap.

Touch menus now reserve the key hint space for the action label. Desktop
menus still show shortcuts. Validate component-specific native
gestures (drag, resize, selection and pinch), accessibility, large text, RTL and
OS-backed services separately. A mobile gallery fixture may simulate an OS
service. It does not establish that the service works on the target OS.

## Image crop and keyboard follow-up

- Android emulator and iPad simulator: the image crop follows a finger drag.
  The gallery constrains the picture to the available width.
- Android emulator at 1600×720 physical pixels: with the software keyboard
  open, scrolling exposes both fields and the Save action in the profile
  dialog. Focusing the second field keeps it visible; Save closes the dialog.
- iPad simulator: moving the floating keyboard onto the former field
  position relocates the profile dialog. The focused field and Save remain
  visible. The host supplies the keyboard rectangle; Ely chooses the layout.
- The layout, media, dialog, and input ownership regression tests passed.
