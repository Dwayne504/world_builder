# Reviewed Type retirement (W04b)

The author approved D03 on 7 October 2026: stop new assignments while preserving
existing Entries, child Types, inherited Fields and authored values. Retirement
is reversible. Permanent deletion and automatic reparenting are outside this
slice.

## Behavior

Open the Category manager, choose a Type and review **Retire Type**. The review
shows counts first, with searchable, paged details for existing Entries, child
Types and default Fields. It includes archived and trashed Entries. Confirmation
uses the reviewed Project revision; a changed Project requires a fresh review.
Restoration uses the same review flow.

A retired Type remains identifiable in managers, existing Entries and filters.
It is no longer offered for a new Entry or as the parent of a new Type. An Entry
already assigned to it can keep that assignment when editing unrelated settings.
An active child Type remains usable and continues to inherit defaults from its
retired parent. Changing an existing Entry away from a retired Type does not make
that Type available for a new assignment back to it; restore the Type first.

This changes the definition's retirement state, not Entry identities, authored
values, Chapter links, relationship facts or the inheritance tree. No supplied
Project or backup is opened or changed by development verification.

Type filters keep the selected option visible while the same Category refreshes;
failed refreshes must not display “All Types” over a still-filtered list. A close
request made during autosave returns to the confirmation when an unfinished
structural draft remains, preserving both the draft and the requested close mode.

## Persistence and dependencies

Schema 15 adds `type_def.retired_at` and database guards for new Entry and child
Type assignments. The ordinary command paths also provide readable validation.
Existing inheritance queries deliberately continue through retired ancestors.
The serialized worker and Project revision guard protect the reviewed command.

This branch includes Type renaming and the existing migration chain: Entry
descriptions (12), workspace pins/recents (13), and Rich Text Fields (14). It is
stacked on the Rich Text branch; do not distribute competing migrations with the
same version. All dependencies remain separate unmerged PRs until the author
chooses to merge them.

## Verification

Backend: all 270 Rust tests passed, with three subprocess helpers exercised by
their parent tests. Formatting, strict all-target Clippy, `cargo check` and diff
checks passed. Six focused integration tests cover assigned and inactive Entries,
descendants, inherited defaults/capabilities, Rich Text preservation, stale and
concurrent reviews, transaction failure, migration rollback and recovery,
reopen, backups and Restore as Copy. An independent SQL guard probe also passed.

Frontend: all 398 tests passed. Six Type-filter and rename regressions passed
again after wrapping the test's asynchronous dialog transitions in React
`act(...)`; that focused run emitted no warnings. Typecheck, lint, formatting
and the production build passed. The pre-existing large bundle advisory remains.
Coverage includes failed and stale reviews, active children, current retired
assignments, filter refreshes and save/close cancellation with pending writes.

Verification completed on 11 October 2026 after the interrupted session resumed.
Sandbox path restrictions prevented the first test launches from reading installed
dependencies; normal-access reruns were used. No local secret scanner or CodeQL
runner/workflow is configured, so no passing local security scan is claimed.

## Windows manual acceptance

Use a disposable Project.

1. Create a parent Type, an active child Type, and Entries using each. Add a
   parent default Field and fill values, including Rich Text.
2. Review retirement. Check the counts, search, paging and inactive Entry markers;
   cancel once, then confirm. Existing assignments and writing must remain.
3. Create an Entry using the active child. Confirm inherited Fields still appear.
   The retired parent must be absent from new assignment and new parent choices.
4. Edit and reopen an existing retired-Type Entry. Keep its Type once, then choose
   another Type. Restore the retired definition and assign it again explicitly.
5. Cancel Save and close while a write is pending; let it succeed or fail. The
   Project must remain open. A close request with an unfinished Category/Type
   draft must return to the confirmation after autosave.
6. Restore a backup as a copy and check the retirement state, child links and
   writing. Check both themes, keyboard focus, long names and display scaling.

Automated component tests do not establish native WebView2 interaction or visual
acceptance. Installer behavior, physical interruption and cloud hydration remain
separate release checks.
