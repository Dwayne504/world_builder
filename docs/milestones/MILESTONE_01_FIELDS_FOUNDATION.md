# Milestone 01 — Task 02B: Fields Foundation

This is the next bounded implementation increment within Architecture Proposal
V2's approved Structure slice. Main at merged PR #10 had schema 2, Categories,
Types and Entries, with no Field storage or authoring commands. The authoritative
behavior is Concept V0.02 §§7, 14; Milestone 01 §9; Architecture V2 §§5–6, 10–11,
15. This note records implementation scope, not a replacement product decision.

## Included

- Optional Short Text, finite Number, Boolean, Choice and Multi-choice values.
  Empty is distinct from authored zero, false, or text such as “Unknown”.
- UUIDv7 definitions, values and choice options scoped to the Project. Matching
  names never merge identities. Value rows use typed columns; choices reference
  stable option IDs with field ownership enforced by foreign keys.
- Separate availability bindings for Categories, Types and ancestor Types, and
  individual Entries. The merged editor deduplicates by definition ID and does
  not materialize empty inherited values.
- Fast local name-and-value creation; promotion by adding availability to the
  same definition; shared definition and choice-option rename; explicit detach,
  retire and restore. Populated values remain visible after template changes.
  Existing retired/detached values remain editable; retired definitions/options
  cannot start new uses. Previously selected retired options remain readable.
- Definition management explicitly identifies shared scope and remains separate
  from Entry value editing. No destructive definition or option deletion.
- Revision-checked worker commands, atomic batch value writes, durable save
  acknowledgement, retained failed/newer drafts, and integration with existing
  Project close and Entry navigation protection. A conflict requires explicit
  reload while keeping drafts, then retry; it never silently overwrites newer
  authoritative data.
- Schema 3 migration through the existing validated external recovery snapshot
  and coordinated manifest publication. Older builds reject schema 3. Backups
  and Restore as Copy preserve internal Field and option identities.

No unresolved product decision was necessary for this increment. Scalar Fields
and Choice accept one value; Multi-choice accepts a set of option identities.
Kinds are immutable in this increment so an unreviewed conversion cannot destroy
authored values. Finite numbers use the backend's typed floating-point storage.

## Explicit follow-up work

The entire Structure slice and the entire Milestone are not complete. Rich Text
Fields require the document envelope/editor work; plain Entry References require
their target and generic backlink behavior. Semantic kind conversions, legacy
value review, configurable scalar cardinality, numeric units, reference
constraints, help text/sections/order/local visibility and further presentation
controls remain follow-ups. None is silently approximated here. Relationships,
Story, Spatial, Search, Timeline, other Capabilities and later milestone systems
remain outside this PR.

## Acceptance and verification

Automated tests cover:

- Schema-2 migration, injected migration rollback, preserved Entry identity and
  data, a validated schema-2 recovery snapshot, and manifest version publication.
  Existing schema-1 migration and lock-recovery tests remain in the full suite.
- Typed values, zero/false versus absent, no empty value rows, reopen persistence,
  and backup/restore with stable internal identities in a new Project.
- Parent-Type availability, local promotion, detachment, Category changes,
  definition retirement/restoration, and retained populated values.
- Choice rename/retirement/restoration, rejection of newly selected retired
  options, wrong-field option IDs, duplicate/single-choice cardinality, and
  stable value identities through edits.
- Stale revisions, cross-Project IDs, missing providers, non-finite/wrong-kind
  input and invalid multi-edit rollback without changing the Project revision.
- UI optionality, local creation, promotion, definition drafts, failed saves,
  explicit conflict reload/retry, and preservation of a newer in-flight draft.
  Field work participates in native-close and navigation guards.

Required commands are the full frontend typecheck/test/lint/format/build suite,
`cargo fmt --check`, `cargo test`,
`cargo clippy --all-targets -- -D warnings`, `cargo check`, and
`git diff --check`. Tests use generated temporary projects; supplied real project
and backup originals are never opened or modified. No dependencies were added.

Local verification passed: 82 frontend tests and 112 Rust tests, TypeScript,
ESLint, Prettier, the production frontend build, Rust formatting, Clippy with
warnings denied, Cargo check, and Git whitespace checks. No repository secret
scanner or CodeQL workflow/CLI is available; a supplemental credential-pattern
scan of staged text changes is not a replacement for those tools.

## Windows manual checklist

Use disposable projects or copies:

1. Add local text, number (including zero), and Boolean (including No) values.
   Close/reopen and check values. Clear a value and confirm it is optional.
2. Promote a local field to a Type, create another Entry of that Type, and verify
   one shared definition with independent values. Rename and detach the field;
   populated values survive while empty inherited presentations disappear.
3. Change an Entry's Category/Type. Old populated values remain marked detached;
   new template Fields become available without creating authored empty data.
4. Create Choice/Multi-choice options; select, rename, retire and restore an
   option. Existing selections remain readable, and retired options cannot be
   newly selected on another Entry.
5. Retire/restore a definition and verify stable values. Test duplicate visible
   field names to confirm that identities are independent.
6. Type a value and immediately navigate or use the OS close shortcut. A pending
   write must finish before closing; failed/unsaved values require retry or
   explicit discard. Unsubmitted definition forms must also block silent exit.
7. Open a disposable schema-2 Project, verify the external recovery snapshot,
   then back up and restore the upgraded Project as an independent copy.

Native packaged GUI behavior and real process-kill/power-loss tests are manual
verification, separate from the automated simulated interruption tests.
