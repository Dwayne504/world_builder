# Focused Entry workspace

This slice responds to the Field-manager and reading-surface feedback after PR #16.
Concept V0.02, Milestone 01 §§9 and 27, and Architecture Proposal V2 remain the
model. The architecture already approves Entry-local Field visibility. The user
explicitly chose **Delete from this Entry only**; shared definitions and other
Entries must survive. No supplied real Projects or backups were opened or changed.
Tests create disposable Projects or use synthetic browser data.

## Behavior

- **Manage fields** opens a wider dialog. Its left table lists current Fields,
  applicable Category/Type defaults (including ancestor Types), and Hide/Delete
  actions. Its right table lists other reusable definitions with an Add action.
  Provider labels use plain names such as Human, without Type/Category prefixes.
  Values are omitted from these tables. Clicking a Field name opens a separate
  shared-definition editor; unfinished edits survive closing either dialog.
- **Hide** saves an Entry-local visibility choice and preserves the Field value,
  shared definition, and defaults. **Show hidden fields** on the Entry page
  temporarily reveals hidden values for reading/editing; it does not change the
  saved choice. **Show** in Manage fields permanently unhides that Field.
- **Delete** opens a review naming the current Entry, Field, and value. The explicit
  confirmation is simply **Delete / Cancel**, with no folder setup. The backend
  first creates a verified recovery backup outside the package.
  Only then does one transaction delete this Entry's value/choice selections and
  local availability, and record a local opt-out of inherited defaults. Existing
  Category/Type bindings and other Entries are untouched. **Add** restores local
  availability and visibility with an empty value. Recover the prior value with
  **Restore Backup as Copy**; no automatic replacement of a live Project occurs.
- Deletion uses the exact reviewed revision. The worker lock spans backup and
  deletion; stale reviews fail before backup, failed backups cannot delete, and
  failed transactions roll back. The existing save controller waits for pending
  deletion during navigation/close. Dismissing a pending configuration dialog
  cannot enable value editing before its acknowledgement. Ordinary value and note
  autosave still permit continuous typing and Tab navigation. No transient
  Save field values button shifts the editor while typing; an explicit retry
  appears only after a failed save, preserving the draft.
- Field values use slender rows. A unit stays beside its number, and clicking any
  part of that control focuses the number. Relationship rows show the meaningful
  forward/inverse phrase and linked Entry, with expandable notes/actions. Technical
  definition names stay in relationship management; lifecycle/conflict warnings
  remain visible. Clear-value controls appear on row hover or keyboard focus and
  remain visible for touch input.
- A faint pointer light crosses the background, controls, and dialogs without
  intercepting clicks. Starship reveals a subtle circle pattern behind the main
  panels near the pointer. Updates are limited to one animation frame and do not
  re-render React. Actual mouse events drive the light even when a mixed-input
  device reports a coarse primary pointer or no hover. The circles sit above
  the background canvas and behind the workspace, avoiding negative stacking.
  Touch events, reduced-motion, and forced-colors modes disable the decoration;
  leaving the window clears it.

## Storage and integration

Automatic recovery uses the current configured Backups directory when usable.
If no usable default exists, it uses `Recovery Backups` within the OS-local
Worldcrafter app-data directory, still organized by Project ID. This never changes
the saved folder preference. Corrupt or newer preferences fail closed; inability
to create a valid backup still blocks deletion. **Backups → Automatic recovery
copies** shows the folder and latest deletion receipt for this session, keeping
paths and maintenance details out of Field authoring. This also works without a
configured Backups directory; no preference or Project schema change is needed
for this follow-up.

Project schema moves from 5 to 6, adding `entry_field_presentation` with local
`hidden` and `removed` flags keyed by Entry ID and Field ID. It creates no empty
value rows and rewrites no existing authored values. Older packages use the
existing external migration-recovery snapshot and transactional upgrade flow.
A failed upgrade preserves schema 5 and can retry; an older app refuses schema 6.

Duplicate merging carries local presentation records to the kept Field. Existing
explicit choices on the kept Field take precedence; otherwise the duplicate's
choice is retained. The merge review explains this and includes Entries affected
only by local removals. Authored values remain governed by the existing reviewed
merge rules and recovery backup. There are no new dependencies, assets, preference
versions, or machine-specific settings.

## Acceptance and verification

- Local visibility survives reopening and Restore as Copy; hiding never changes
  values or another Entry. Invalid/stale requests cannot mutate the Project.
- Deleting Number and Choice data suppresses inherited defaults on this Entry,
  preserves shared definitions/options and other Entries, and re-adds empty.
- Stale, foreign-ID, invalid-backup and injected-transaction failures preserve
  data. Concurrent deletion commits and backs up exactly once. The backup restores
  the complete pre-delete value and local visibility state.
- Schema-5 migration failure rolls back; retry preserves authored values. Duplicate
  merging keeps local presentation choices without orphaned references.
- UI tests cover two tables, ancestor-default context, hidden-field preview,
  explicit unhide/re-add, failed hiding, automatic recovery receipts, deletion
  review/acknowledgement, exact reviewed revisions, pending-save navigation, and
  a calm autosave surface with a retry only on failure.
- Pointer tests cover frame coalescing, touch exclusion, leave/unmount cleanup,
  reduced motion, and forced colors. Existing draft and autosave tests remain.

Passed locally on Windows: `npm run typecheck`, `npm test -- --run` (141 tests,
no React act warnings), `npm run lint`,
`npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo test` (148 tests),
`cargo clippy --all-targets -- -D warnings`, `cargo check`, and `git diff --check`.
The exact file list and PR status are recorded in the PR description.

Headless Edge uses synthetic Tauri responses and delayed writes. It checks both
styles at 1600px and 390px, compact row geometry, numeric whitespace/unit clicks,
manager columns/reflow, nested-dialog focus, Hide/peek/Show, simple Delete/Add,
continued title/value/note typing without a transient save button, recovery
receipts in Backups, and disabled motion in accessibility modes. The mouse-light
check also emulates a coarse primary pointer/no hover.
Screenshots are inspected and remain outside the repository. Packaged WebView2,
screen-reader announcements, and actual Windows display scaling are not covered
by those browser checks.

GitHub secret scanning and push protection are enabled, with no open alerts at
verification. A supplemental staged-diff scan checks credentials, personal paths,
and forbidden artifacts. No local secret-scanner CLI is installed. CodeQL has no
workflow, CLI, or analysis in this repository, so no CodeQL pass is claimed.
The pre-existing content-neutral Cargo.toml modification is excluded.

## Windows manual checklist

Use a new disposable Project or a disposable copy.

1. Open Manage fields. Check current/default indicators, other reusable Fields,
   keyboard navigation, and retained shared-definition drafts after Escape.
2. Hide a populated default Field, reopen the Project, temporarily show hidden
   Fields, then unhide it. Confirm the value and other Entries are unchanged.
3. Delete a Field only from one Entry. Confirm the inherited default stays absent
   after reopening and another Entry keeps its value. Add it back empty, then
   find its recovery copy under Backups, restore it as a copy, and verify the
  original value. Repeat once without a configured Backups directory.
4. Click blank space beside a number; type, pause, continue and Tab. Do the same
   with a relationship note. Check title-bar close while deletion is pending.
5. Switch Storybook/Starship; resize, use 125–200% scaling, test long labels/units,
   and try Windows high contrast/reduced animations. The light must remain faint,
   never intercept input, and stop when the pointer leaves the window.

## Deferred

A full ribbon/toolbar, draggable relationship graph, free-form color customization,
unit conversions, Type deletion/lifecycle decisions, shared-definition permanent
deletion, recent Projects, Category pages/filters, and unrelated milestone systems
remain deferred. This change concentrates controls in the existing section
headers and management dialogs.
