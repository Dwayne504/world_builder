# Relationships foundation — PR #13

This is the next bounded part of the approved Ownership slice after Categories,
Types, Entries, Fields, and the workspace cleanup. It follows Concept V0.02,
Milestone 01 §10, and Architecture Proposal V2. It does not complete the whole
Ownership slice: configurable relationship-backed Fields remain a follow-up.
No new product decision is introduced.

## Delivered behavior

- Reusable, Project-scoped relationship definitions with stable IDs, a working
  name, directed forward/inverse labels or a symmetric label, and optional soft
  maximum expectations per side. Direction is fixed once created; changing the
  meaning of existing participants needs a separately reviewed migration.
- One registered relationship instance and two participant rows form the single
  semantic record. The two Entry views derive from that record. Names are display
  labels, not keys. Renaming or reorganizing an Entry preserves the connection.
- A compact Relationships section in each Entry supports navigation to the other
  Entry, a continuously saved plain-text note, and reversible end/restore. Past
  and inactive connections remain visible in a disclosure. Definition retirement
  stops new use and preserves records and their notes.
- Creation supports an existing target or an optional-name, optional-Category
  stub created in the same transaction. Missing Category means Uncategorized.
  The editor stays on the source Entry after creation.
- Conflicting active relationships are all retained and flagged from either
  Entry. Explicit replacement ends only the selected current connections on the
  chosen side and creates the replacement atomically. Their IDs and notes remain
  in history. There is no automatic owner selection, deletion, or repair.
- An equivalent active connection with the same definition and participants is
  rejected, including reversed symmetric duplicates. Self-connections are
  structurally valid and shown once. Distinct definitions retain distinct meaning
  even if their labels happen to match.

## Persistence and save contract

Schema 4 adds definitions, registered instances, and participant storage, with
foreign keys and participant-kind validation. Reads derive inverse views,
participant labels, and conflict warnings. There are no independent owner values
or persisted conflict caches. New creation accepts active Entries from this
Project only. An unresolved participant snapshot can be displayed without
inventing a new Entry or repairing the relationship.

Every operation runs on the existing serialized Project database worker inside
one revision-checked transaction. The snapshot returned to the UI is constructed
before commit; success is returned only after commit. Stub creation, replacement,
identity registration, participants, and the global revision commit together.
An error rolls the operation back. Upgrading schema 1–3 uses the existing validated
external pre-migration snapshot and recoverable manifest publication.

Relationship and Field controls share only acknowledged revisions from this Entry
editor, so unrelated local saves can follow one another without unnecessary stale
revision failures. External changes still fail the revision check. An explicit
reload preserves local drafts. Failed and newer note drafts survive acknowledgement
of an older request. Creation and definition forms are preserved when a dialog is
hidden; cancelling is explicit. Relationship saving joins the existing Entry
navigation and native-close protection.

## Acceptance coverage

Rust integration tests use synthetic Projects in disposable temporary directories:
one record/two views; identity through rename and Category change; notes and
end/restore; visible conflicts; explicit replacement; rollback after an injected
participant-write failure with no orphan stub; symmetric duplicate detection;
self-connections; definition editing/retirement; stale and foreign IDs; invalid
expectations; competing revision-checked commands; unresolved participant
snapshots; reopening; Restore as Copy with internal IDs retained; and interrupted
schema-3 upgrade, external recovery snapshot, and retry.

Frontend tests cover inverse navigation, conflict visibility and explicit
selection, atomic quick creation, hidden and failed drafts, note acknowledgement,
close waiting, explicit reload/retry, newer notes, symmetric configuration,
end/restore identity, native close during creation, and a Field write following a
relationship commit.

Verified on Windows on 2026-09-29:

- `npm run typecheck`
- `npm test -- --run` (102 tests; the JSON reporter was used to keep diagnostics concise)
- `npm run lint` (no warnings)
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` (121 tests)
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Headless Edge checks passed for both perspectives, target navigation, dialog
keyboard focus, Escape draft retention, narrow layout without horizontal overflow,
and light/dark rendering, with no page errors. These checks used synthetic command
responses and do not replace packaged Tauri manual verification.

GitHub reports secret scanning and push protection enabled. No local secret-scan
CLI or repository scan script is supplied; a supplemental added-line credential
and private-path check is used before committing. GitHub CodeQL default setup is
`not-configured`, and no CodeQL CLI or workflow is supplied. CodeQL was not run.
All real supplied Projects/backups remained untouched. The pre-existing local
Cargo.toml state is excluded from this PR.

## Windows manual checklist

Use a new disposable Project or a disposable copy, never a supplied original.

1. Open a schema-3 test copy. Confirm its Entries and Fields are intact after the
   schema-4 upgrade, then close and reopen it.
2. Open Thron. Under Manage relationships, create Ownership, with labels `owns`
   and `is owned by`, and expected maximum per target `1`.
3. Add a relationship and choose Create an Entry here. Name it Singularity Blade.
   Follow its name: it should show `is owned by Thron`. Edit a note, wait for Saved,
   navigate back, and verify the note from both sides.
4. Give the Blade another owner without selecting a replacement. Both connections
   should remain and show warnings. From the Blade, explicitly select the old
   owner to end while connecting a new one. Confirm only the selected connection
   ended and its note remains under past relationships.
5. End and restore a connection; rename and retire/restore its definition. Verify
   existing data survives. Try an equivalent active connection and read the error.
6. Hide a partly filled relationship dialog using Escape/Done, reopen it, then
   cancel explicitly. Check keyboard focus, scrolling at 800×600, and dark mode.
7. Edit a note and use Back, Close Project, and the window close button. Check that
   pending saves are awaited and failed/unfinished work remains protected.
8. Make a manual backup, Restore as Copy, and verify both relationship directions,
   notes, and ended history in the restored Project.

## Explicit follow-ups

- Configurable relationship-backed Fields/projections and fuller Ownership editing.
- Category/Type endpoint constraints, advanced definition metadata and migrations.
- Rich Text and other remaining Field kinds; generic navigational reference Fields.
- Recent Projects on Home: roughly three names that open directly, without showing
  their paths as the main labels.
- Entry filtering by Category; dedicated Category browsing with Type filters.

The last two items preserve the user's feedback for later UI work. No Category
pages, recent-project persistence, Story, Spatial, Search, Timeline, graph view,
multi-party relationship editor, or other milestone features are added here.

## Files changed

- `README.md`
- `app/src-tauri/src/application/service.rs`
- `app/src-tauri/src/domain/mod.rs`
- `app/src-tauri/src/domain/relationships.rs`
- `app/src-tauri/src/domain/structure.rs`
- `app/src-tauri/src/lib.rs`
- `app/src-tauri/src/persistence/migrations.rs`
- `app/src-tauri/src/persistence/migrations/0004_relationships.sql`
- `app/src-tauri/src/persistence/mod.rs`
- `app/src-tauri/src/persistence/relationships.rs`
- `app/src-tauri/src/persistence/worker.rs`
- `app/src-tauri/src/tauri_boundary/commands.rs`
- `app/src-tauri/tests/fields_foundation.rs`
- `app/src-tauri/tests/relationships_foundation.rs`
- `app/src/App.css`
- `app/src/App.test.tsx`
- `app/src/App.tsx`
- `app/src/EntryFieldsPanel.test.tsx`
- `app/src/EntryFieldsPanel.tsx`
- `app/src/EntryRelationshipsPanel.test.tsx`
- `app/src/EntryRelationshipsPanel.tsx`
- `app/src/api.ts`
- `app/src/types.ts`
- `app/src/useEntryFields.ts`
- `app/src/useEntryRelationships.ts`
- `docs/milestones/MILESTONE_01_RELATIONSHIPS_FOUNDATION.md`
