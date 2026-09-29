# Sketchbook workspace — PR #15

This is a presentation and interaction pass following PR #14 and the user's
screenshots. It follows Concept V0.02's common Entry editor and optional
authoring model, Milestone 01's editor/configuration separation, and approved
Architecture Proposal V2. No new domain behavior or storage schema is introduced.

## Visible changes

- The Entry title is the editor: click the heading text or reach it with Tab and
  type. There is no duplicate name field underneath. Existing continuous saving
  still applies; Enter also submits without leaving the title. Empty titles keep
  the `[Unnamed Entry]` placeholder and remain valid.
- The committed Category appears above the title and the optional Type below it.
  Unapplied Category/Type settings do not mislabel the current Entry.
- Numbers and units share one quantity control, such as `48 years`. The unit is
  also an accessible description of the numeric input. Ordinary values no longer
  repeat definition-kind metadata; Field creation/management retains those controls.
- Fields use a heading action and **Add field** dialog, matching Relationships.
  Project Home uses **Add Entry**. Categories and defaults has a wider overview
  with separate **Add Category**, **Add Type**, **Add default field**, and
  **Reuse field** dialogs. Optional inline Category/Type creation remains inside
  the Entry creation dialog.
- The workspace uses available window width. Category groups form columns; wide
  Entry editors put Fields and Relationships side by side. Narrow windows stack
  sections and controls without losing access to forms.
- Warm, light paper, ink-colored text, serif titles, fine rules, and subtle browned
  edges replace the green theme. The requested light appearance is deliberate in
  both system color schemes. Decorative edges are CSS-only, ignore pointer input,
  and disappear in forced-colors mode. Keyboard focus remains visible.

## Draft and save behavior

Closing a dialog with Done or Escape hides its form without discarding it. A
visible Continue action reopens unfinished work. Explicit Cancel clears that
draft. Successful creation closes the form after acknowledgement.

Entry creation drafts now also participate in the Project's close protection,
including a chosen Category/Type and unfinished inline creation. Existing Entries
cannot be opened while that creation draft is unfinished; finish or cancel it
first. This prevents hiding the draft behind a different Entry's save controller.
Pending structural writes still must settle before closing; failures and drafts
remain visible. Title/Field/note acknowledgements preserve newer edits and focus.

No supplied real Projects or backups were opened or modified. Tests use synthetic
data and disposable Projects. The existing local Cargo.toml modification remains
excluded from this work.

## Verification

Local Windows checks:

- `npm run typecheck`
- `npm test -- --run` — 120 tests, with the JSON reporter; no React act warnings
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` — 127 tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Regressions cover in-place title saving/failure/newer text, unnamed titles,
committed Category/Type context, accessible unit association, dismissed and failed
Field drafts, retained Entry drafts/native close, and parent/child Category forms.
Existing save, navigation, backup, recovery, and template tests remain included.

Headless Microsoft Edge used synthetic Tauri responses, including delayed writes.
Checks exercised title/note typing and caret retention, Field Tab navigation,
inline unit geometry, dialog focus restoration and containment, reopening drafts,
Category defaults and Type assignment, the full-width 1600-pixel layout, 390-pixel
reflow, light appearance under a dark system preference, and forced colors.
Screenshots were inspected. No browser errors occurred.

GitHub secret scanning and push protection are enabled, with no open alerts at
verification. A supplemental staged scan checks credential patterns, private
paths, and forbidden artifacts. No local secret-scanner CLI is installed.
CodeQL is not configured and no CodeQL workflow/CLI exists, so CodeQL analysis is
not claimed. GitHub CI status is reported separately on the PR.

Native packaged Tauri/WebView2 behavior, screen-reader announcements, and actual
Windows display scaling are not independently verified by these browser tests.

## Windows manual checklist

Use a new test Project or a disposable copy.

1. Click an Entry title and edit it, including a pause for autosave. Continue
   typing without clicking again. Press Enter, reopen, and check the final name.
   Clear it and confirm an unnamed Entry is still usable.
2. Check Category above and Type below the title, including an untyped Entry.
   Change settings without applying, then apply, and verify labels follow the
   committed assignment.
3. Check `48 years` and a long custom currency beside its number. Edit/clear
   values and move with Tab through the Fields while they save.
4. Open each Add dialog. Enter a draft, press Escape, and resume it. Test a failed
   save and native close with an unfinished draft. Cancel must be explicit;
   submitted writes must finish before closing.
5. In Categories, open and dismiss a child form. Confirm keyboard focus returns
   to its launcher and the unfinished default remains available to continue.
6. Resize from a wide desktop to a narrow window. Check scrolling, long names and
   units, Windows text/display scaling, and high-contrast mode. The ordinary
   palette should remain light regardless of the Windows dark-mode setting.

## Deferred

Recent Projects, Category pages/Type filters, deeper Category/Type management,
unit conversions/localized number formatting, relationship projections, and
other milestone systems remain outside this PR. There are no new image assets,
fonts, dependencies, theme preferences, migrations, or backend commands.
