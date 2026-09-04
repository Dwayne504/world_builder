# The Worldcrafter — Milestone 01 Architecture Proposal V2

**Status:** Approved for Milestone 01 implementation. **Scope:** this document defines the technical architecture; implementation proceeds strictly vertical-slice by vertical-slice as scoped in `MILESTONE_01_IMPLEMENTATION.md`, starting with the Project Trust Foundation slice.

## 1. Executive verdict and revision summary

V1 established useful boundaries but is not safe to implement as written: it uses unscoped identifiers in several tables, proposes destructive relationship UPSERTs, makes Roles part of a chapter link, uses `synchronous=NORMAL` despite the Saved promise, places backups inside the live package, and leaves assets, migration recovery, unresolved targets, and transaction ownership underspecified.

V2 selects a Project-per-package SQLite design. Every durable ID is a UUIDv7 *scoped by the opened Project package*. A shared internal identity registry provides record-family identity without making Story Units or Relationships into Entries. One dedicated database worker owns each open Project's connection and all transactions. Relationships, plain references, story links, and structural parentage are distinct stores with distinct deletion rules.

## 2. Authoritative requirements and non-goals

This proposal implements the behavior in Concept V0.02 and `MILESTONE_01_IMPLEMENTATION.md`; V0.02 supersedes V0.01. Product decisions in the correction brief—restore-as-copy identity, relationship versus reference, soft cardinality, and role-free Story links—are fixed.

Milestone 01 excludes timelines/calendars, Event Capability, maps/coordinates, graph visualization, storyboards, full story hierarchy, multi-party editing, claim/canon systems, historical-state queries, cross-Project live sharing, collaboration/sync, plugins, AI generation, and publishing layout/export pipelines. Extension seams below do not implement those systems.

## 3. Architecture overview

```
React UI/state -> typed Tauri commands -> application commands/queries
                                      -> domain validation -> ProjectDbWorker
                                      -> SQLite authored tables + derived-index work
                                      -> package/assets, backup, migration services
```

The package is the Project boundary, not a shared database tenant. Thus `E17` in P1 and P2 is safe; application references always carry `{project_id, record_kind, record_id}` at IPC, navigation, search, backup, and external-boundary interfaces. Within an open package, foreign keys use its local IDs; the worker rejects a command whose supplied Project ID differs from the manifest/database Project ID.

## 4. Identity and Project package model

`manifest.json` is the portable package identity authority; its `project_id` must match `project_meta.project_id`. IDs are immutable UUIDv7 strings, never names, paths, category, or ordering.

```
<display-name>.wcproj/
  manifest.json                 # project_id, format_version, DB/doc/asset versions
  data/project.sqlite           # WAL/shm are live operational files, not portable copies
  assets/<asset-id>/<filename>  # managed relative paths
  staging/                      # recoverable incomplete asset imports
```

`record_identity(record_id PK, kind CHECK(entry|story_unit|relationship_instance), workspace_state CHECK(active|archived|trashed), lifecycle_changed_at, created_at)` is the sole Archive/Trash authority for registered identifiable records. Family tables have `id PK REFERENCES record_identity(record_id)` and never duplicate Archive/Trash columns. This does not flatten families: only Entries own Category/Fields/Capabilities; Story Units own order/documents; Relationship Instances own participants. Tags use `tag_assignment(record_id, tag_id)` only for taggable registered kinds. Future kinds register a new kind and table, not an Entry subtype.

Restore copies the snapshot package, assigns only a new manifest/project-meta Project ID, removes live lock/WAL artifacts, validates and opens it. It preserves all internal IDs and links because the complete reference graph remains within P2. Changed Project metadata: project ID, created/restored timestamps, `restored_from_backup_id` and `restored_from_project_id`; name/path may be chosen by user. Automatic backups are immutable snapshots outside the package, never independently editable Projects. Future import/merge/sharing must add explicit `origin_project_id`, source ID, and translation mapping; it must not infer identity from matching names.

## 5. Concrete domain/persistence model

All tables have `created_at`, `updated_at`, and `revision INTEGER NOT NULL`; only `record_identity` has workspace lifecycle state, while reusable definitions/options may have `retired_at`. `project_id` is deliberately absent from per-package rows; package/worker scope enforces it. `ON DELETE` policies below prevent accidental cascades. Authored tables are authoritative; `derived_*` and FTS tables are rebuildable.

| Entity/table (owner) | keys, constraints, and important indexes |
|---|---|
| `project_meta` (package) | singleton `project_id UNIQUE`, `schema_version`, `format_version`, `last_committed_revision`; reject mismatched manifest |
| `category` (structure) | `id PK`; `name`; `is_uncategorized UNIQUE WHERE true`; no delete while Entries reference it |
| `entry` (Entry) | `id PK/FK record_identity`; `category_id FK RESTRICT`; `type_id FK RESTRICT NULL`; label, description document ID; `INDEX(category_id,type_id)` |
| `type_def` (structure) | `id PK`; `category_id FK RESTRICT`; `parent_type_id FK RESTRICT NULL`; `retired_at`; acyclic parent command; index category |
| `capability_def` (system) | fixed `base`, `spatial`; not author-extensible in M1 |
| capability defaults (Capability) | `category_capability_default(category_id FK RESTRICT, capability_id FK RESTRICT, PRIMARY KEY(category_id,capability_id))`; `type_capability_default(type_id FK RESTRICT, capability_id FK RESTRICT, PRIMARY KEY(type_id,capability_id))` |
| `entry_capability` (Capability) | `(entry_id FK RESTRICT, capability_id FK RESTRICT, PRIMARY KEY(entry_id,capability_id))`; the row is the Entry-owned actual set, with no ambiguous source/provenance column |
| aliases (Entry) | `entry_alias(id PK, entry_id FK CASCADE, alias_text, normalized_alias, UNIQUE(entry_id,normalized_alias), INDEX(normalized_alias))`; aliases are authored alternative labels, not Tags or derived-index rows; an alias is removed only by its explicit alias-delete command or its owning Entry's confirmed permanent deletion |
| `field_definition` (Fields) | `id PK`; semantic kind, cardinality, reference constraints, presentation metadata, `retired_at`; never stores an owner polymorphically |
| `field_availability` (templates) | `(field_id, provider_kind, provider_id) PK`; provider is category/type/entry-local; parent Types are resolved recursively; detach deletes this binding, retire changes definition state |
| scalar field values (Fields) | `field_value(id PK, entry_id FK RESTRICT, field_id FK RESTRICT, value_kind, text/number/bool/document_id, legacy_payload NULL)`; unique `(entry_id,field_id,ordinal)`; typed columns, not undifferentiated JSON |
| choices (Fields) | `choice_option(id PK, field_id FK RESTRICT, label, retired_at)` and `field_choice_value(value_id FK, option_id FK RESTRICT)`; options retain stable IDs |
| plain references (Fields) | `entry_reference_value(id PK, entry_id FK RESTRICT, field_id FK RESTRICT, target_record_id NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT, target_kind, unresolved_snapshot JSON, ordinal)`; target FK is nullable only after explicit permanent deletion |
| relationship definitions (Relationships) | `relationship_definition(id PK, directed, forward/inverse labels, source/target constraints, expected_targets_per_source, expected_sources_per_target, retired_at)`; both expectations are nullable positive soft expectations |
| instances/participants (Relationships) | `relationship_instance(id PK/FK identity, definition_id FK RESTRICT, semantic_state CHECK(active|ended), ended_at NULL, note, metadata_document_id NULL)`; `relationship_participant(instance_id FK CASCADE, slot CHECK(source|target), record_id NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT, record_kind, unresolved_snapshot, PRIMARY KEY(instance_id,slot))`; exactly two slots in M1 |
| story (Story) | `story_unit(id PK/FK identity, kind CHECK(chapter), title, reading_rank)`; `UNIQUE(reading_rank)`, independent of chronology |
| story links/roles (Story) | `story_link(id PK, story_unit_id FK RESTRICT, entry_id NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT, unresolved_snapshot, UNIQUE(story_unit_id,entry_id))`; `story_link_role(link_id FK CASCADE, role_id FK RESTRICT, PRIMARY KEY(link_id,role_id))`; role definitions have stable IDs and `retired_at` |
| status/tags (supporting) | named `status_system`, `status_value`, and assignments by stable IDs; `tag`, `tag_assignment`; neither is Category, Type, lifecycle, or Role |
| spatial (Spatial) | `spatial_node(entry_id PK/FK entry, primary_parent_id FK spatial_node RESTRICT NULL)`; indexed parent. This is authored structural truth, not a Relationship. |
| Project navigation (workspace) | `project_pin(record_kind,record_id REFERENCES record_identity ON DELETE RESTRICT, pinned_at, PRIMARY KEY(record_kind,record_id))`; `project_recent(record_kind,record_id REFERENCES record_identity ON DELETE RESTRICT, last_accessed_at, access_rank, PRIMARY KEY(record_kind,record_id), INDEX(last_accessed_at))` |
| application navigation (application home) | `app_recent_project(project_id, package_path, last_opened_at, last_accessed_at, PRIMARY KEY(project_id,package_path), INDEX(last_accessed_at))`; it is outside all Project packages |

Definitions, values, availability, and presentation are separate. Status is creative controlled state; Archive/Trash are the `record_identity.workspace_state`; Retire applies only to reusable definitions/options. Derived tables include `derived_backlink`, `derived_search_state`, FTS, and optional cached closure, all marked by source revision.

## 6. Fields and template availability

Availability is the union of Category, Type and its ancestors, and Entry-local bindings. It creates no empty `field_value`. Local visibility is `entry_field_presentation(entry_id,field_id,hidden,section_override)` and does not mutate a shared definition. Promotion changes/adds an availability binding while retaining the definition and values.

A relationship-backed field has `field_projection(field_id PK, relationship_definition_id FK, perspective source|target, label/section override)` and **no** value/reference row. Mother, Parent, Birthplace, Current Owner, Member Of, Current Location, and Based In must use this projection. `See Also`, Related Research Entry, Reference Entry, and template/example links use `entry_reference_value`: generic incoming backlinks only.

Retiring a choice/definition stops new selection/availability but preserves values. Removing a template binding preserves populated values as detached historical values. A semantic kind migration records `field_value.legacy_payload` plus review state; convertible values are explicitly converted, incompatible values remain readable and excluded from invalid new input. Restrictive type/target/cardinality changes constrain future edits and flag existing violations. Destructive deletion is a separate confirmed command with a recovery snapshot.

## 7. Relationships and projections

Definitions define expected source/target meanings, target constraints, directed/symmetric presentation, labels, grouping, and two soft directional expectations. `expected_targets_per_source` is selected by a source-perspective field (for example, Owns); `expected_sources_per_target` is selected by a target-perspective field (for example, Current Owner). Thus one Ownership definition can expect many Objects for each owner and one owner for each Object; reversing those values represents many-to-one instead. A symmetric definition uses the same expectation from either endpoint (and requires compatible values if both columns are populated); its command canonicalizes participant ordering by ID for duplicate detection and displays the same label at both ends. Binary participant rows are the sole participant truth now and later: adding roles/more slots extends `relationship_participant`; no parallel A/B store is introduced.

Relationship projections query the same instance by definition and participant slot. A current projection considers only instances whose identity workspace state is `active` and whose `semantic_state` is `active`; it resolves an active instance's Archived or Trashed target and visibly marks that target. Backlinks are semantic and show direction/context. Constraints are validated in the command, but authored violations persist and remain queryable. For an expected-one projection, query every matching current instance, return all targets plus `Expected one; found N`; never choose one. Archived/Trashed instances remain authored and inspectable but are not automatically counted as current semantic facts.

**Relationship-backed Field edit transaction:** application command resolves projection perspective, validates IDs/project/definition and future target constraint, `BEGIN IMMEDIATE`, inserts or changes the selected instance/participant only when unambiguous, updates index dirty revision, commits, then emits revision. For zero targets it creates one instance; for one it edits that instance; for multiple it returns a conflict requiring an instance selection or Replace/Keep-both action—never UPSERTs.

**Explicit Replace transaction:** command requires displayed current instance IDs and revision; `BEGIN IMMEDIATE`, re-checks them, transitions each selected old instance's `semantic_state` from `active` to `ended` and sets `ended_at`, creates the replacement instance, records undo/audit intent, marks derived data dirty, commits. This relationship lifecycle is distinct from workspace Archive/Trash and supplies the minimal current-versus-former distinction without a historical-ownership UI. Keep-both creates another instance after a warning. Cancel writes nothing. A later restrictive definition does not delete old violations.

## 8. Capabilities and Spatial

`category_capability_default` and `type_capability_default` bind default Capabilities to their providers. Entry creation calculates the union of its Category defaults, its selected Type and ancestor-Type defaults, and mandatory Base, then materializes one `entry_capability` row per Capability in the new Entry-owned set. A Capability supplied by multiple defaults still creates only that one row, so no ambiguous source value is stored. An Entry can explicitly add/remove compatible actual Capabilities through its owned set. Later Category/Type default changes affect future Entry creation only: they do not silently add, remove, or destroy an existing Entry Capability or its dependent authored data. Removing Spatial is refused until its parent, children, and spatial-dependent data are explicitly repaired; it never drops rows implicitly.

Tortuga is `Category=Creatures`, `Type=World Turtle`, `Base+Spatial`; Northern Shell → City of Arak → Temple are spatial nodes. One nullable parent makes roots valid. Reparent uses one transaction: validate both nodes are spatial and active policy-compatible; recursive ancestor query from proposed parent must not find child; update only child parent; mark ancestry/search/backlinks dirty; commit. It changes neither descendants nor Thron.

Thron's Current Location is an ordinary direct relationship to Temple. Arak/Northern Shell/Tortuga are recursive derived ancestry. Spatial queries distinguish direct children from descendants and direct location from anywhere-within. Adjacency/gates/connections are ordinary Relationships, not another edge table.

## 9. Story Units and rich documents

Loose Chapters are `story_unit(kind=chapter)` and need no Book. `reading_rank` is an ordered sequence and never chronology. Manuscript, Plan, and Notes each reference separate `rich_document` rows:

`rich_document(id PK, owner_kind, owner_id, area, document_schema_version, canonical_json, plain_text, word_count, revision, migration_state)`, unique `(owner_kind,owner_id,area)`.

The envelope validates a versioned TipTap/ProseMirror AST server-side before commit, extracts plain text and Unicode-aware word count, and records document schema migration. If migration fails, preserve original JSON read-only, show recovery/export text, do not overwrite it, and block edits to that document only.

Story-link command: validate Chapter and Entry; `BEGIN IMMEDIATE`; insert/find the unique `story_link`; insert zero or more stable Role assignments on that same link; dirty indexes; commit. A zero-Role link is valid. Archive/Trash preserves it and target resolution markers. Permanent Entry deletion nulls `story_link.entry_id` and snapshots identity; permanent Chapter deletion removes its owned links after confirmation. Story Usage is a derived direct backlink; ancestor setting context is separately labelled derived.

## 10. Persistence, autosave, and transactions

Each open Project has one Rust `ProjectDbWorker` owning one `rusqlite` connection on a dedicated blocking thread. Tauri async handlers enqueue typed commands; UI never opens SQLite. The worker serializes writes (`BEGIN IMMEDIATE`) and services controlled snapshot reads between commands or through read-only connections opened after WAL setup. Application command modules own transactions; repositories cannot independently commit.

Set `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=FULL`, `busy_timeout`, and checkpoint policy. A committed FULL-synchronous transaction is the ordinary durable Saved contract. Every mutation carries `expected_revision`; stale writes reject with current snapshot/revision, never last-writer-wins. Worker increments global and record revisions in its transaction.

Text edits debounce locally (for example 250–500 ms); blur, navigation, close request, and explicit retry flush immediately and await commit. Structural commands are immediate. `Saved` appears only after commit acknowledgement; pending, saving, failed, and retrying are distinct. Normal close waits for flush or warns/cancels close. Disk-full/permission/IO failures preserve dirty UI state, show “Changes are not being saved,” stop Saved claims, retry on request, and write a best-effort encrypted/local emergency manuscript buffer outside the package. That buffer is recovery assistance, not durable Project storage.

## 11. Assets, backups, restore, locking, and migrations

Asset import copies to `staging/<operation-id>`, hashes it, allocates `asset_id`, chooses `assets/<id>/<sanitized-original>` (same names never collide), then transactionally records `asset_manifest`; rename/move completes atomically. Failed staging is cleaned/recoverable. Missing assets retain asset ID/path/last metadata and render a missing marker, not a crash. No absolute managed paths.

`lock.json` contains process ID, host, random lease token, heartbeat, and package ID. Open obtains atomic create/exclusive lock; stale recovery requires failed heartbeat/age confirmation and records recovery. Cloud folders are supported as filesystems, not sync/collaboration: lock conflicts, unexpected manifest/database revision, or duplicated package IDs warn and require opening a copied package.

Backups live in application backup storage, e.g. `.../Worldcrafter/backups/<project-id>/`, outside `.wcproj`. Live backup is a worker command: quiesce writes, use SQLite Online Backup API to temporary DB (never copy only main DB in WAL), checkpoint/close snapshot, copy assets from a consistent command boundary, write manifest/checksums, validate SQLite integrity and hashes, atomically rename a compressed `.wcbackup`. Rolling retention is explicit (last N hourly/daily plus M manual); manual backups are named/pinned until user removal. Corrupt snapshots are quarantined and never offered as valid restoration.

Migration checks manifest format, DB schema, rich-document schema, and asset format. Newer unsupported formats refuse write/open (future read-only is separate). Before irreversible migration, make a validated recovery snapshot. Run DB migration in a transaction where SQLite permits; otherwise stage a new DB/package, validate, atomically swap with rollback marker, then update manifest last. Interrupted migration finds the marker, validates old/new, rolls back or resumes deterministically; no half-version opens writable.

## 12. Lifecycle and unresolved references

`record_identity.workspace_state` has exactly one effective state: `active` appears in ordinary workspace listings, `archived` is hidden from those listings but remains resolvable with an Archived marker, and `trashed` is recoverably removed from ordinary listings but remains resolvable with a Trashed marker. Archive and Trash commands transition that single state; restoring either returns it to `active` and preserves links. An Archived/Trashed Relationship Instance is not counted by a current semantic projection, whereas an active instance still resolves an Archived/Trashed participant with its marker. Semantic `relationship_instance.semantic_state` independently distinguishes `active` current facts from `ended` former facts; Archive never means Former or Ended. Retire stops new definition use. Undo is immediate command history, not backup. Permanent deletion is explicit after dependency review and recovery point.

References to registered identities use a nullable `record_id` FK to `record_identity` plus a stored `record_kind`; SQLite cannot enforce the polymorphic family table directly, so application/domain validation verifies that the registry kind equals the stored kind on creation or retargeting, and the database uses triggers/checking commands to reject a mismatched kind. `entry_reference_value.target_record_id`, `relationship_participant.record_id`, and `story_link.entry_id` are `ON DELETE RESTRICT`; ordinary SQL deletion therefore cannot bypass the explicit permanent-deletion command. That command, in one transaction, snapshots `{former_id, kind, label, deleted_at}` into every nullable semantic reference (including those three), nulls each target FK, updates derived state, and only then removes the target identity. Archive and Trash never null those keys. The referring relationship/link remains identifiable and repairable; same-name recreation never repairs it. Owned alias rows and other explicitly reviewed owned data are removed only as part of that confirmed permanent-deletion command. Structural dependencies are different: Category must be reassigned; spatial children must be reparented/rooted or explicitly subtree-deleted; Story ordering is compacted/repaired. Trash does not cascade through Spatial by default. Parent permanent deletion requires explicit direct-child action and never silently cascades.

## 13. Search, Explore, backlinks, and derived indexes

Search is fast approximate lookup; Explore is structured query/browse. Search indexes Entry/Story exact labels and IDs first, authored `entry_alias.normalized_alias` values, tags, typed field values/choice labels, plain-reference labels, relationship semantic context, description/manuscript extracted text. Aliases resolve to their one owning Entry identity; they are not Tags and remain authoritative outside the derived index. Results group Entries, Story Units, structured connections, and text mentions; direct structured links never masquerade as prose mentions. Ranking boosts exact ID/name then aliases before FTS, with Unicode tokenization but no assumed English Porter stemming.

Explore filters Category, Type, Capability, Tags, relationship definition/context, direct location, anywhere-within recursive location, and Story involvement. Backlinks derive from relationship participants, plain references, Story links, and optionally tagged kinds; direct and derived Spatial/Story context are separately marked.

Every committed authored mutation sets `derived_index_state(source_revision, indexed_revision, schema_version, dirty)` in its transaction. The indexer applies work after commit; queries fall back to authoritative SQL when dirty. A version mismatch, integrity check failure, or recovery invokes a transactional/full rebuild from authored tables. Breadcrumbs, recursive ancestry, counts, Saved View results, FTS, and backlinks are never sole truth.

## 14. Module boundaries and dependency direction

| Module | ownership |
|---|---|
| `domain` | IDs, invariants, validation specifications; no SQLite/UI |
| `application` | named commands, expected revisions, starts/commits worker transaction, maps failures/Saved acknowledgement |
| `application_home` | application-level Recent Projects storage and retention; never writes Project-authored data directly |
| `persistence` | schemas, repositories, `ProjectDbWorker`; no product decisions |
| `migrations` / `backup_recovery` / `package_assets` | format transitions, snapshots, locks, assets |
| `documents` | envelope validation/migration/plain text |
| `indexing` | derived updates/rebuilds only |
| `tauri_boundary` | authenticated/project-scoped command DTOs/events |
| `react_ui` | drafts, tabs/history/context/filter restoration; calls commands only |

For every mutation the domain validates invariant, application owns the transaction and failure mapping, persistence performs rows, indexing marks dirty inside and updates after commit, and UI decides display state only. UI components never write tables directly.

`project_pin` is author-controlled and ordered by `pinned_at`; `project_recent` is automatically upserted on record access, ordered by `last_accessed_at`, and bounded by a configurable retention limit through explicit eviction of its oldest rows. Both are Project-local and reference registered records by `{record_kind,record_id}`. At every application boundary those references also carry the open `project_id`, yielding `{project_id,record_kind,record_id}`. Application-home `app_recent_project` is outside packages, updates timestamps on open/access, orders by `last_accessed_at`, and applies an application-configured bounded retention policy; stale paths are retained as repairable recent items until explicitly removed. Session state holds tabs, Back/Forward context, cursor/scroll when practical, and Explore filters; restart restoration is desirable and explicitly not crash recovery.

## 15. Vertical-slice implementation plan

| Slice | outcome/concepts | persistence and failure demonstration | exclusions |
|---|---|---|---|
| 1 Trust spikes | prove UUID/package, worker/FULL durability, TipTap envelope, backup API | kill/IO/stale-write/lock tests | product UI |
| 2 Tiny Project | create/open/rename/reopen a real package | manifest/DB migrations, close flush, corrupt refusal | fields/relationships |
| 3 Structure | Categories, Types, Fields, safe evolution | typed values/options, detach/retire/migration review tests | relationships |
| 4 Ownership | Thron owns Blade projections | one relationship, conflict/replace tests | Spatial |
| 5 Tortuga | Base+Spatial and hierarchy | transaction cycle/reparent/crash tests | maps |
| 6 First Step | Chapter, documents, links/roles | document migration/save fallback tests | passage links |
| 7 Navigate | Search/Explore/backlinks/navigation | dirty/rebuild/ranking/filter tests | graph |
| 8 Recovery | lifecycle/assets/backups/restore | deletion, missing asset, snapshot/restore tests | sync |
| 9 Torture | canonical release proof | kill/migration/backup/rename/reopen tests | deferred systems |

## 16. Failure-oriented testing strategy

Domain/property tests cover ID scope, no name repair, participant cardinality warnings, no duplicate projection values, field evolution, capability ownership, Spatial cycles, ordering, and lifecycle transitions. SQLite integration tests inject transaction failure between every structural step, stale revisions, FULL-durability close/reopen, disk-full/permission errors, WAL backup, corrupt snapshot, lock/stale lock, asset staging, and interrupted migration. Tauri/UI tests cover conflict choices, visible unresolved/archived/trashed markers, Saved failures, navigation state, and canonical workflows. Fixture tests rebuild every derived index and compare authoritative-query results.

## 17. Canonical acceptance-scenario walkthroughs

1. **World:** one Ownership instance has source Thron and target Blade; source shows Owns and target projection shows Current Owner. Tortuga remains Creatures/World Turtle/Base+Spatial; its hierarchy is four `spatial_node` rows. Thron has one direct Current Location instance to Temple. One Chapter has three Story links: Thron has POV+Main Character Roles, Temple has Setting+Primary Setting, Blade has none. Renames/reparent/lifecycle/reopen preserve IDs.
2. **Conflict:** two active Ownership instances target Thron and Leopold. Current Owner projection returns both and warning. Generic create retains both; only explicit Replace ends selected old instances through relationship lifecycle and creates the selected replacement atomically. Archiving either target retains resolution with its marker; archiving an instance excludes it from the current projection without deleting it.
3. **Copy:** validated P1 backup restored to P2 rewrites only Project metadata. E17/E42, Relationship participant rows, Story links, definitions, and all internal FKs resolve in P2. P1/P2 workers/package locks permit independent edits.
4. **Plain reference:** A's `entry_reference_value` points at B and generates a generic backlink. It creates no Relationship projection. Converting to semantic meaning is an explicit migration: create definition/instance, then remove or retain-with-review the plain reference—never both hidden stores.

## 18. Changes from Proposal V1

V2 replaces V1's global-looking IDs with Project-scoped identity and restore semantics; fixes Story link/Role normalization; prohibits destructive relationship UPSERT; replaces JSON-only values; uses participant truth now; specifies typed references/unresolved snapshots; selects one rusqlite worker model; raises durability to FULL; moves backups outside packages and uses Online Backup API; adds locking/assets/migration recovery; and makes index/Saved ownership transactional.

## 19. Remaining risks and engineering spikes

Validate Tauri close-event flushing, filesystem atomic rename/lock behavior on supported cloud folders, `rusqlite` backup/interrupt handling, TipTap schema migration fidelity, FTS multilingual behavior, and asset snapshot consistency. These are engineering spikes, not product-decision blockers. No new product decision is required for the approved scenarios.

## 20. Architecture approval checklist

- [ ] Project-scoped stable identity and Restore-as-Copy preserve all internal IDs.
- [ ] Entries, Story Units, Relationships, fields, references, roles, statuses, and Spatial truth remain distinct.
- [ ] Semantic relationships are stored once; plain references are separate and generic.
- [ ] Soft conflicts survive; explicit replacement is atomic and non-silent.
- [ ] Story links exist with zero-to-many Roles.
- [ ] Template/capability evolution preserves authored data.
- [ ] Capability defaults, aliases, directional cardinality, and Project/application navigation state have concrete authoritative persistence.
- [ ] Spatial parentage is acyclic authored structure; ancestry is derived.
- [ ] Saved means committed FULL-synchronous durable Project storage.
- [ ] Backups/migrations/assets/locks have the stated recovery mechanisms.
- [ ] Workspace lifecycle, semantic relationship lifecycle, unresolved-reference integrity, and derived index rebuilds are concrete.
- [ ] Implementation follows vertical slices and retains all stated non-goals.
