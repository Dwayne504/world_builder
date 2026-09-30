# Relationship Fields

This bounded slice continues Ownership after the Relationships foundation and
desktop navigation work. It follows Concept V0.02 §§7.7 and 8, Milestone 01
§§9–10 and 45, and approved Architecture Proposal V2 §§6–7. It does not complete
Milestone 01 or begin Spatial, Story, timelines, or graph visualization.

## Authoring behavior

- **Relationship** is a Field kind configured with an existing Relationship
  Definition and the meaning seen from this Entry's side. A directed definition
  offers its forward and inverse labels; a symmetric definition has the same
  meaning at either endpoint. Field names remain presentation, never identity.
- An Entry-local Field or Category/Type default uses the existing availability
  model, including ancestor Types. Promoting or reusing the Field retains its
  stable definition. An empty available Field creates no relationship or scalar
  value until the author connects an Entry.
- The Field displays all matching current relationships. Current means both an
  active workspace record and an active semantic relationship; ended, Archived,
  and Trashed instances are excluded from the current value. An active
  relationship's inactive or unresolved participant remains visibly marked.
- Choosing a target for an empty Field creates one canonical relationship.
  **Change** on a connected Entry edits that particular instance while retaining
  its ID and note. When several connections exist, each has its own Change
  action; no arbitrary owner or other target is selected for the author.
- Soft-cardinality warnings remain visible beside all conflicting connections.
  Existing Relationships controls provide **End**, **Restore**, explicit
  replacement, and creation that keeps the existing connections. Replacement
  ends only the reviewed selected records and retains their history; it differs
  from editing one instance's target.
- The target picker searches active Entries and can create a named stub with an
  optional Category. Stub creation and connection or retargeting commit together,
  and the author stays on the original Entry.
- Existing notes are readable in the Field. Editing notes and relationship
  lifecycle still uses the full Relationships controls.

## Presentation and preservation

The Entry's normal Relationships section omits a current instance already shown
in a visible Field. **Show all relationships, including those in Fields**
reveals those rows for note and lifecycle editing. The Project Relationships page
always provides the complete filterable relationship view. Suppression uses
instance IDs, not matching names or labels, and never removes the stored record.

**Hide** preserves the Field's configuration and all connections; temporarily
showing hidden Fields or choosing Show restores their presentation. A connection
not currently displayed in a Field remains available in the normal Relationships
section. **Remove from Fields** removes only this Entry's Field presentation and
opts out of inherited availability. It does not end connections, delete notes,
change another Entry, or change shared defaults. Adding the Field back displays
the same current relationships again.

Scalar **Delete** and duplicate-field **Merge** do not operate on Relationship
Fields. The existing approval for deleting a scalar value from only one Entry
does not imply permission to delete a semantic fact also shown at its other end.
Ending a connection remains an explicit relationship operation.

Retiring a configured Field keeps its populated projection visible while a
binding still applies. Unbinding it, changing Category/Type so its binding no
longer applies, or removing its local presentation returns display to the
canonical Relationships section; the fact and its note survive. A retired Field
does not become available for new use. Relationship-definition retirement also
preserves existing connections and prevents new target changes until restored.
Changing a projection's relationship definition or perspective is not an implicit
semantic conversion in this slice.

## Persistence and save contract

Schema 7 adds `field_projection`, keyed by Field ID with a Relationship
Definition ID and source/target perspective. It extends the Field kind constraint
and forbids scalar `field_value` storage for a projection. Canonical relationship
instances and their participant rows remain the only target truth; reads derive
Field contents from those records rather than caching an independent owner or
other target value.

The existing validated external pre-migration recovery snapshot protects the
schema upgrade. The migration rebuilds the constrained definition table inside
the migration transaction, checks foreign-key integrity before commit, and
restores connection enforcement settings on success or failure. Database and
manifest version publication follows the existing recoverable upgrade path.
Older builds reject newer schema versions rather than opening them for writing.

Projection commands use the serialized Project worker, reviewed Project revision,
and one transaction. A stale revision, invalid instance, incompatible command,
duplicate active connection, or failed participant/stub write does not partially
apply the operation. Save success follows commit acknowledgement. Field and
Relationship views refresh from acknowledged revisions without treating the
derived presentation as another authored value. Pending and failed editing
participates in the existing navigation and desktop-close guards.

## Verification status

Verified on Windows on 2026-09-30:

- Frontend typecheck, all 184 tests, lint, formatting check, and production build passed.
- Rust formatting check, all 175 tests, Clippy with warnings denied, and Cargo check passed.
- Repository whitespace check passed. GitHub secret scanning and push protection are
  enabled with no open secret alerts. No local secret scanner or CodeQL CLI/workflow
  is configured; the code-scanning API reports no available analysis (404).
- Focused synthetic-fixture tests for one-instance/two-view identity, no scalar
  storage, current and historical lifecycle, all-conflict visibility, selected
  retargeting, symmetric/self connections, stale writes, duplicate rejection,
  rollback, inherited availability, local presentation changes, and recovery.
- UI tests for definition configuration, per-instance editing, inline stubs,
  notes/warnings, visible-only deduplication, the full-view toggle, draft/error
  retention, navigation, and close protection.

The isolated native Windows build was exercised with a newly generated Project:
inverse Ownership Field configuration, inline target creation, linked navigation
and inverse view, note autosave/readback, and the OS close guard for an unfinished
target change. Desktop testing exposed a stale sidebar count after inline creation;
the correction is covered by the app integration test. The remaining cases in the
manual checklist below are covered by focused automated tests, but were not all
repeated through native UI interaction.

Use generated disposable Projects for all checks. Supplied real Projects and
backup originals remain read-only evidence and must never be opened for writing.
Headless browser checks with synthetic IPC do not independently verify the
packaged Windows desktop; report those methods separately.

## Windows manual checklist

Use a new disposable Project or a disposable copy, never an original:

1. Create Characters and Objects, an owner and an object. Define directed
   Ownership (`owns` / `is owned by`) with expected sources per target `1`.
   Add an object Field named Current owner, kind Relationship, using the inverse
   meaning. Choose the owner, navigate both ways, and verify one connection.
2. Add a note through **Show all relationships, including those in Fields**.
   Change the owner from the Field; verify the same note remains and the old and
   new owner views agree with the object's current owner.
3. Create another current owner through Relationships. Both owners and the
   warning must appear. Change one specific connection; verify the other stays.
   Exercise explicit End/Restore and replacement separately.
4. Promote the Field to a Category or Type default. An existing eligible Entry
   shows an empty optional Field without an invented connection. Create a target
   inline with an optional Category and confirm the source editor stays open.
5. Hide, temporarily reveal, Show, remove from Fields, and add the projection
   again. Check that the connection and note remain available and that only
   visible Field contents suppress duplicate relationship rows. Retire/restore
   or detach the shared Field and inspect the preserved connection.
6. Check the Project Relationships page and a symmetric connection. Rename a
   participant and confirm the current label updates everywhere without creating
   another relationship. Use duplicate visible names to check disambiguation.
7. Close/reopen after Saved. Back up and Restore as Copy; verify projection
   configuration, identities, notes, and ended history. On a disposable older
   package, check successful schema upgrade and its external recovery snapshot.
8. Leave a target change unfinished and use Back or the OS close shortcut;
   confirm explicit finish/cancel handling. A failed or stale write must preserve
   the draft and require a deliberate retry/reload rather than silently saving.

## Explicit follow-ups

Plain navigational Entry Reference Fields remain separate from semantic
relationships. Rich Text, remaining Field metadata, semantic kind/projection
conversion, projection merging, permanent relationship deletion, Category/Type
endpoint constraints, and fuller general lifecycle UI are not implemented here.
Spatial containment, Chapters/prose, indexed search, tabs/pins, timelines,
fictional calendars, historical-state queries, graphs, and multi-party editing
remain later work. This slice adds no hard-coded Ownership names or category
behavior and does not infer relationship identity from matching labels.
