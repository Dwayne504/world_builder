# Appearance and reviewed Field merging — PR #16

This responds to feedback after PR #15. The approved scope is two app styles,
whole-row numeric editing, clearer Field identification/reuse, reviewed duplicate
merging with a recovery backup, and a less exposed Close Project action.

The user approved reuse suggestions plus an explicit merge review. Name equality
alone never changes stored identity. Concept V0.02, Milestone 01 §§9 and 27, and
Architecture Proposal V2 remain the governing model. No supplied real Project
or backup was opened or modified; all data tests generate disposable Projects.

## Behavior

- **Appearance** is available on Home and inside a Project. **Storybook** adds
  warm parchment and a stitched leather header; **Starship** uses muted dark
  panels, cyan accents, sans-serif titles, and brushed metal. CSS decorations do
  not intercept clicks; layouts reflow and support forced colors. Switching
  styles keeps the workspace and writing drafts mounted.
- A native label makes the entire number/unit control focus its input, including
  whitespace and the unit. Units still sit immediately beside the number.
- **Close Project** moves into Project settings. Existing unsaved-change and
  in-flight-write protection remains in effect, including native window close.
- Field selectors show kind, unit, named Category/Type/Entry bindings and retired
  state. Indistinguishable definitions get a list-local copy number, never a raw
  ID suffix. Entry management groups fields on this Entry separately.
- Add Field/default forms suggest reusing same-name active definitions. Reuse
  binds the selected existing ID, preserves every Entry's values, and allocates
  no duplicate definition. Entry creation drafts with an initial value must be
  cleared explicitly before reuse; they are not silently lost.
- **Categories → Combine duplicate fields** reviews two same-name definitions
  across the whole Project. It shows scopes, affected Entries (including empty
  available Fields and historical values), both values and the proposed result.
  The chosen target ID and existing target values survive; missing target values
  move from the source with their IDs and creation times; identical overlaps
  become one value. Source bindings join the target's bindings. The duplicate
  definition is removed only after an explicit confirmation and recovery backup.
- Different values, kinds, units, or retired definitions block merging. Choice
  and Multi-choice merging also blocks: option mapping is not defined yet.
  These cases never guess a value, unit conversion, or option identity.
- The backend checks Project revision and compatibility before creating a
  verified external backup, holds the Project worker lock through backup and
  merge, and rechecks inside one SQLite transaction. Stale reviews and backup
  failures cannot write; a failed transaction rolls back all Field changes.
  Undo is **Restore Backup as Copy**, not a destructive overwrite of the Project.
- The existing Field retirement action is labeled **Remove field from new use**
  with an explanation that values remain and restoration is available.

## Preferences and storage

App preferences move from schema 1 to schema 2 with an enum appearance setting.
Version 1 reads normalize in memory to Storybook while preserving both configured
folders; reads do not rewrite a valid file. The next explicit preference change
publishes schema 2 with the existing serialized, Windows-safe recovery protocol.
Invalid or missing schema-2 appearance values and newer versions fail closed.
A failed theme save leaves the prior style active and offers a visible retry.
Reset from Home also adopts the default appearance in the same session.

Project schema remains 5. No dependency, asset, or local configuration files are
added. The pre-existing content-neutral Cargo.toml change is excluded.

## Acceptance checks

- Click blank space or the unit, edit the number, and Tab while autosave runs.
- Switch both styles, preserve writing drafts, restart to the saved style, and
  show preference errors without rewriting corrupt/newer files.
- Reuse an Entry Field as a Type default, preserving its ID/value.
- Merge compatible duplicate numbers, preserving data on multiple Entries.
  Restore the recovery backup and recover the two pre-merge definitions.
- Reject different values/units/kinds, foreign IDs, self-merges, stale reviews,
  failed backup destinations, and injected transaction failures. Concurrent
  requests commit once and create one recovery backup.
- Show human-readable context instead of ID fragments; require review and
  confirmation before merging; report successful commit even if refresh fails.
- Keep Project close out of the main toolbar and preserve unsaved safeguards.

## Verification

All checks below passed locally on Windows:

- `npm run typecheck`
- `npm test -- --run` — 130 tests (JSON reporter), no React act warnings
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` — 135 tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

The suite includes new preference migration/concurrency, Field merge/recovery,
appearance, reuse, and management UI coverage.

Headless Microsoft Edge tests use synthetic Tauri responses and delayed saves.
They verify both styles, 1600px and 390px layouts, whole-row/unit clicks, inline
unit geometry, keyboard navigation, retained Field/Entry/default drafts,
relationship-note and title caret retention, dialog focus, forced colors, and
Close Project placement. A separate browser flow verifies the merge review,
confirmation, delayed write, recovery-path feedback, refreshed Entry and narrow
dialog. Screenshots are inspected; generated media stays outside
the repository. Native packaged WebView2, screen-reader announcements, and real
Windows scaling still need manual verification.

GitHub secret scanning and push protection are enabled; no open alerts were
reported. A supplemental diff scan checks credential patterns, personal paths,
and forbidden artifacts. No local secret-scanner CLI is installed. CodeQL is
not configured and no CodeQL workflow or CLI exists; no CodeQL result is claimed.

## Windows manual checklist

Use a fresh Project or disposable copy, never source evidence.

1. In a `48 years` Field, click blank space on the right and then the unit. Type,
   pause for autosave, continue, Tab to another Field, and reopen to check values.
2. Switch Storybook/Starship; resize and check long names/units, contrast,
   125–200% display scaling, and Windows high-contrast mode. Restart and check
   the selected appearance. Keep an Entry draft while switching styles.
3. Add local Age, then add Age as a Human default using **Reuse Age**. Confirm
   one Field appears with its original value.
4. On a disposable Project with duplicate Age definitions, review and merge them.
   Check affected Entries, definitions, defaults and values. Restore the generated
   backup as a copy and verify the two original definitions and values remain.
5. Give the duplicates different values or units; verify merging is blocked.
   Change a value after opening a review; verify that stale review cannot commit.
6. In Manage fields, remove a Field from new use, confirm filled-in values stay,
   and restore it. Close Project from settings with clean and unfinished work.

## Pending / deferred

The second product question (reversible Field/Type retirement versus also adding
reviewed permanent deletion) is still awaiting an answer. This change does not
introduce Type deletion or a new Type lifecycle rule. Existing Field retirement
is made easier to discover.

Custom colors, Choice option mapping during merge, unit conversions, recent
Projects, Category pages/Type filters, and other milestone systems are deferred.
