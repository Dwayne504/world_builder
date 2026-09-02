# The Worldcrafter — Milestone 01 Architecture Proposal V2

**Status:** Architecture revision for approval. **Scope:** architecture only; no application implementation is authorized by this document.

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

`record_identity(record_id PK, kind CHECK(entry|story_unit|relationship_instance),
 created_at, archived_at, trashed_at)` is the shared identity infrastructure. Family tables have `id PK REFERENCES record_identity(record_id)`. This does not flatten families: only Entries own Category/Fields/Capabilities; Story Units own order/documents; Relationship Instances own participants. Tags use `tag_assignment(record_id, tag_id)` only for taggable registered kinds. Future kinds register a new kind and table, not an Entry subtype.

Restore copies the snapshot package, assigns only a new manifest/project-meta Project ID, removes live lock/WAL artifacts, validates and opens it. It preserves all internal IDs and links because the complete reference graph remains within P2. Changed Project metadata: project ID, created/restored timestamps, `restored_from_backup_id` and `restored_from_project_id`; name/path may be chosen by user. Automatic backups are immutable snapshots outside the package, never independently editable Projects. Future import/merge/sharing must add explicit `origin_project_id`, source ID, and translation mapping; it must not infer identity from matching names.

## 5. Concrete domain/persistence model

All tables have `created_at`, `updated_at`, `revision INTEGER NOT NULL`, and relevant `archived_at`, `trashed_at`, or `retired_at` columns. `project_id` is deliberately absent from per-package rows; package/worker scope enforces it. `ON DELETE` policies below prevent accidental cascades. Authored tables are authoritative; `derived_*` and FTS tables are rebuildable.

| Entity/table (owner) | keys, constraints, and important indexes |
|---|---|
| `project_meta` (package) | singleton `project_id UNIQUE`, `schema_version`, `format_version`, `last_committed_revision`; reject mismatched manifest |
| `category` (structure) | `id PK`; `name`; `is_uncategorized UNIQUE WHERE true`; no delete while Entries reference it |
| `entry` (Entry) | `id PK/FK record_identity`; `category_id FK RESTRICT`; `type_id FK RESTRICT NULL`; label, description document ID; `INDEX(category_id,type_id)` |
| `type_def` (structure) | `id PK`; `category_id FK RESTRICT`; `parent_type_id FK RESTRICT NULL`; `retired_at`; acyclic parent command; index category |
| `capability_def` (system) | fixed `base`, `spatial`; not author-extensible in M1 |
| `entry_capability` (Capability) | `(entry_id, capability_id) PK`; source (`category_default`,`type_default`,`entry_explicit`) is provenance only; actual row is Entry-owned |
| `field_definition` (Fields) | `id PK`; semantic kind, cardinality, reference constraints, presentation metadata, `retired_at`; never stores an owner polymorphically |
| `field_availability` (templates) | `(field_id, provider_kind, provider_id) PK`; provider is category/type/entry-local; parent Types are resolved recursively; detach deletes this binding, retire changes definition state |
| scalar field values (Fields) | `field_value(id PK, entry_id FK RESTRICT, field_id FK RESTRICT, value_kind, text/number/bool/document_id, legacy_payload NULL)`; unique `(entry_id,field_id,ordinal)`; typed columns, not undifferentiated JSON |
| choices (Fields) | `choice_option(id PK, field_id FK RESTRICT, label, retired_at)` and `field_choice_value(value_id FK, option_id FK RESTRICT)`; options retain stable IDs |
| plain references (Fields) | `entry_reference_value(id PK, entry_id FK, field_id FK, target_record_id NULL, target_kind, unresolved_snapshot JSON, ordinal)`; target FK is deliberately nullable only after explicit permanent deletion |
| relationship definitions (Relationships) | `relationship_definition(id PK, directed, forward/inverse labels, source/target constraints, expected_cardinality, retired_at)` |
| instances/participants (Relationships) | `relationship_instance(id PK/FK identity, definition_id FK RESTRICT, note, metadata_document_id NULL)`; `relationship_participant(instance_id FK CASCADE, slot CHECK(source|target), record_id NULL, record_kind, unresolved_snapshot, PRIMARY KEY(instance_id,slot))`; exactly two slots in M1 |
| story (Story) | `story_unit(id PK/FK identity, kind CHECK(chapter), title, reading_rank)`; `UNIQUE(reading_rank)`, independent of chronology |
| story links/roles (Story) | `story_link(id PK, story_unit_id FK RESTRICT, entry_id NULL, unresolved_snapshot, UNIQUE(story_unit_id,entry_id))`; `story_link_role(link_id FK CASCADE, role_id FK RESTRICT, PRIMARY KEY(link_id,role_id))`; role definitions have stable IDs and `retired_at` |
| status/tags (supporting) | named `status_system`, `status_value`, and assignments by stable IDs; `tag`, `tag_assignment`; neither is Category, Type, lifecycle, or Role |
| spatial (Spatial) | `spatial_node(entry_id PK/FK entry, primary_parent_id FK spatial_node RESTRICT NULL)`; indexed parent. This is authored structural truth, not a Relationship. |

Definitions, values, availability, and presentation are separate. Status is creative controlled state; Archive/Trash are lifecycle columns; Retire applies only to reusable definitions/options. Derived tables include `derived_backlink`, `derived_search_state`, FTS, and optional cached closure, all marked by source revision.

## 6. Fields and template availability

Availability is the union of Category, Type and its ancestors, and Entry-local bindings. It creates no empty `field_value`. Local visibility is `entry_field_presentation(entry_id,field_id,hidden,section_override)` and does not mutate a shared definition. Promotion changes/adds an availability binding while retaining the definition and values.

A relationship-backed field has `field_projection(field_id PK, relationship_definition_id FK, perspective source|target, label/section override)` and **no** value/reference row. Mother, Parent, Birthplace, Current Owner, Member Of, Current Location, and Based In must use this projection. `See Also`, Related Research Entry, Reference Entry, and template/example links use `entry_reference_value`: generic incoming backlinks only.

Retiring a choice/definition stops new selection/availability but preserves values. Removing a template binding preserves populated values as detached historical values. A semantic kind migration records `field_value.legacy_payload` plus review state; convertible values are explicitly converted, incompatible values remain readable and excluded from invalid new input. Restrictive type/target/cardinality changes constrain future edits and flag existing violations. Destructive deletion is a separate confirmed command with a recovery snapshot.

## 7. Relationships and projections

Definitions define expected source/target meanings, target constraints, directed/symmetric presentation, labels, grouping, and soft expected cardinality. A symmetric command canonicalizes participant ordering by ID for duplicate detection but displays the same label at both ends. Binary participant rows are the sole participant truth now and later: adding roles/more slots extends `relationship_participant`; no parallel A/B store is introduced.

Relationship projections query the same instance by definition and participant slot. Backlinks are semantic and show direction/context. Constraints are validated in the command, but authored violations persist. For expected-one projections, query every matching active/archived/trashed instance, return all targets plus `Expected one; found N`; never choose one.

**Relationship-backed Field edit transaction:** application command resolves projection perspective, validates IDs/project/definition and future target constraint, `BEGIN IMMEDIATE`, inserts or changes the selected instance/participant only when unambiguous, updates index dirty revision, commits, then emits revision. For zero targets it creates one instance; for one it edits that instance; for multiple it returns a conflict requiring an instance selection or Replace/Keep-both action—never UPSERTs.

**Explicit Replace transaction:** command requires displayed current instance IDs and revision; `BEGIN IMMEDIATE`, re-checks them, archives/ends (not deletes) the selected relationship(s) using explicit lifecycle metadata, creates the replacement instance, records undo/audit intent, marks derived data dirty, commits. Keep-both creates another instance after a warning. Cancel writes nothing. A later restrictive definition does not delete old violations.

## 8. Capabilities and Spatial

Category/Type bindings supply defaults only when creating an Entry. `entry_capability` is copied/created as the actual set, so later template edits never remove a Capability or its data. Base is guaranteed. Removing Spatial is refused until its parent, children, and spatial-dependent data are explicitly repaired; it never drops rows implicitly.

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

Archive hides active listings but resolves targets with a marker. Trash is recoverable and likewise resolves; restore clears only lifecycle state and preserves links. Retire stops new definition use. Undo is immediate command history, not backup. Permanent deletion is explicit after dependency review and recovery point.

Permanent semantic target deletion snapshots `{former_id, kind, label, deleted_at}` into relationship participant, plain reference, story link, and future generic-reference rows, then nulls the target FK in the same transaction. The referring relationship/link remains identifiable and repairable; same-name recreation never repairs it. Structural dependencies are different: Category must be reassigned; spatial children must be reparented/rooted or explicitly subtree-deleted; Story ordering is compacted/repaired. Trash does not cascade through Spatial by default. Parent permanent deletion requires explicit direct-child action and never silently cascades.

## 13. Search, Explore, backlinks, and derived indexes

Search is fast approximate lookup; Explore is structured query/browse. Search indexes Entry/Story exact labels and IDs first, aliases, tags, typed field values/choice labels, plain-reference labels, relationship semantic context, description/manuscript extracted text. Results group Entries, Story Units, structured connections, and text mentions; direct structured links never masquerade as prose mentions. Ranking boosts exact ID/name then aliases before FTS, with Unicode tokenization but no assumed English Porter stemming.

Explore filters Category, Type, Capability, Tags, relationship definition/context, direct location, anywhere-within recursive location, and Story involvement. Backlinks derive from relationship participants, plain references, Story links, and optionally tagged kinds; direct and derived Spatial/Story context are separately marked.

Every committed authored mutation sets `derived_index_state(source_revision, indexed_revision, schema_version, dirty)` in its transaction. The indexer applies work after commit; queries fall back to authoritative SQL when dirty. A version mismatch, integrity check failure, or recovery invokes a transactional/full rebuild from authored tables. Breadcrumbs, recursive ancestry, counts, Saved View results, FTS, and backlinks are never sole truth.

## 14. Module boundaries and dependency direction

| Module | ownership |
|---|---|
| `domain` | IDs, invariants, validation specifications; no SQLite/UI |
| `application` | named commands, expected revisions, starts/commits worker transaction, maps failures/Saved acknowledgement |
| `persistence` | schemas, repositories, `ProjectDbWorker`; no product decisions |
| `migrations` / `backup_recovery` / `package_assets` | format transitions, snapshots, locks, assets |
| `documents` | envelope validation/migration/plain text |
| `indexing` | derived updates/rebuilds only |
| `tauri_boundary` | authenticated/project-scoped command DTOs/events |
| `react_ui` | drafts, tabs/history/context/filter restoration; calls commands only |

For every mutation the domain validates invariant, application owns the transaction and failure mapping, persistence performs rows, indexing marks dirty inside and updates after commit, and UI decides display state only. UI components never write tables directly.

Persistent application-home storage (separate from Projects) holds Recent Projects by package/project ID/path and timestamps. Project tables hold Recent records and Pins by `{record_kind,record_id}`. Session state holds tabs, Back/Forward context, cursor/scroll when practical, and Explore filters; restart restoration is desirable and explicitly not crash recovery.

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
2. **Conflict:** two Ownership instances target Thron and Leopold. Current Owner projection returns both and warning. Generic create retains both; only explicit Replace archives/ends selected old instance(s) and creates the selected replacement atomically.
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
- [ ] Spatial parentage is acyclic authored structure; ancestry is derived.
- [ ] Saved means committed FULL-synchronous durable Project storage.
- [ ] Backups/migrations/assets/locks have the stated recovery mechanisms.
- [ ] Lifecycle/unresolved references and derived index rebuilds are concrete.
- [ ] Implementation follows vertical slices and retains all stated non-goals.
