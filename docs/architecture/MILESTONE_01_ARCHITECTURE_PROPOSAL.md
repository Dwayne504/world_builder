# The Worldcrafter — Milestone 01 Technical Architecture Proposal

**Document:** `MILESTONE_01_ARCHITECTURE_PROPOSAL.md`  
**Status:** Architecture Proposal for Review  
**Target Milestone:** Milestone 01 — The First Real Worldcrafter  
**Contracts & Sources of Truth:**
- Engineering Contract: `docs/milestones/MILESTONE_01_IMPLEMENTATION.md`
- Product Concept Source of Truth: `docs/concept/The_Worldcrafter_Concept_V0.02.docx`

---

## Executive Summary & Design Invariants

This proposal defines the technical architecture for **Milestone 01** of **The Worldcrafter**, a local-first desktop application for worldbuilding and story writing.

The central goal of Milestone 01 is to prove the core product loop:
> **Create material → structure it only where useful → connect it → navigate it from multiple directions → write with that material nearby → close and reopen the application without losing or corrupting anything.**

This architecture is designed around the hard invariants defined in the product specification:
1. **Stable Identity:** Visible names are never identifiers. All durable records (Projects, Entries, Categories, Types, Field Definitions, Relationship Definitions, Story Units, Tags) use immutable internal UUIDv7 identifiers. Renaming or moving records never breaks references.
2. **Optionality:** Supported structure is optional. Uncategorized entries, entries without types, empty fields, incomplete stubs, and loose chapters are native first-class states. The system never blocks the user with mandatory questionnaires or completion percentages.
3. **Single Source of Truth for Relationships:** A semantic relationship is stored exactly once in persistence. Forward views, inverse views, and relationship-backed fields are projections of a single underlying relationship instance.
4. **Template Evolution Safety:** Category and Type templates define available structure, but do not own authored values. Modifying or removing templates never silently deletes authored data.
5. **Derived Context Non-Duplication:** Derived data (spatial ancestry breadcrumbs, backlinks, recursive location contexts, search indexes) is computed via projection queries or rebuildable caches. Authoritative authored structure remains the single source of truth.
6. **Structural Atomicity & Crash Safety:** Multi-table operations (spatial reparenting, template promotion, relationship creation, deletion flows) execute inside ACID SQLite transactions. The application never leaves half-applied invalid states after an interruption or crash.
7. **Reversibility by Default:** Structural operations favor reversible states (Archive, Trash, Type changes, reparenting). Permanent deletion is explicit and exceptional.
8. **Manuscript Integrity:** Manuscript text is authored prose. Record renames update linked metadata projections without mutating raw authored manuscript prose.

---

## 1. Recommended Technology Stack

We recommend the following concrete technology stack for Milestone 01.

| Layer / Concern | Recommended Technology | Primary Justification |
| :--- | :--- | :--- |
| **Desktop Framework** | **Tauri 2.0** (Rust Core + Web View) | Local-first requirement; extremely lightweight memory footprint (~30–50 MB RAM vs 200 MB+ for Electron); tiny binary distribution (~10–15 MB); native OS filesystem access and thread safety via Rust core. |
| **UI Framework** | **React 19 + TypeScript + Tailwind CSS** | Proven UI component model for multi-panel desktop interfaces (sidebars, context drawers, multi-tab state); strict type safety matching backend domain via Specta / IPC bindings. |
| **Languages** | **Rust** (Core engine, persistence, transactions, search, backups) + **TypeScript** (UI rendering, view state, rich text editor) | Rust provides memory-safe, crash-resilient local persistence, ACID transactions, and thread-safe indexing; TypeScript provides rapid UI layout and rich-text ecosystem integration. |
| **Local Persistence** | **SQLite 3** (via `rusqlite` in Rust) with **WAL mode** | The gold standard for local-first desktop applications. Single-file ACID database per project (contained inside a `.wcproj` package folder). Instant local queries, WAL crash durability, atomic transactions. |
| **Rich-Text Editor** | **ProseMirror / TipTap** | Pure JSON AST document model; decorator-based inline mentions without mutating raw text; robust transaction history (undo/redo); preserved scroll/cursor state. |
| **Search & Indexing** | **SQLite FTS5** + Rust CTE Query Engine | Embedded full-text search with BM25 ranking; zero external engine overhead; instant search across manuscript and lore; rebuildable from primary database tables. |
| **Testing Frameworks** | **Cargo Test** (Rust domain) + **Vitest / RTL** (UI) + **Playwright** (E2E) | Fast domain unit testing for business rules and persistence; automated E2E tests for desktop workflow acceptance. |
| **Build & Tooling** | **Vite** + **Cargo** + **Tauri CLI** | Standardized, high-speed build pipeline for cross-platform bundling (macOS, Windows, Linux). |

### Evaluation of Repository Context
The repository currently contains specification documents and no existing codebase. Establishing Tauri 2.0 + Rust + React + SQLite gives Worldcrafter a durable foundation capable of scaling into a substantial desktop application for long-form authors without performance bottlenecks.

---

## 2. High-Level Application Architecture

The application is structured into clear architectural layers with unidirectional data flow and strict boundary separation.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        PRESENTATION LAYER (UI)                         │
│   React 19 + Tailwind CSS + TipTap Editor + Multi-Tab & View State    │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │  IPC Bridge (Tauri Commands & Events)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       APPLICATION SERVICE LAYER                        │
│   Use Case Services (Command Handlers, Query Handlers, Event Bus)      │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                           DOMAIN CORE ENGINE                           │
│   Pure Rust Entities: Entry, Category, Type, Capability, Field,        │
│   Relationship, Spatial Graph, Story Unit, Lifecycle Rules             │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
         ┌─────────────────────────┴─────────────────────────┐
         ▼                                                   ▼
┌────────────────────────────────┐         ┌─────────────────────────────┐
│  PERSISTENCE & INTEGRITY LAYER │         │    SEARCH & INDEX ENGINE    │
│  SQLite 3 (WAL) + Repositories │         │  FTS5 Indexer + Backlinks  │
│  Transaction Manager           │         │  Spatial Ancestry CTEs      │
└────────────────────────────────┘         └─────────────────────────────┘
```

### Module Boundaries and Responsibilities

1. **Presentation Layer (UI - TypeScript/React):**
   - Renders Project Home, Entry Editor, Spatial Surface, Chapter Editor, Context Panels, Search/Explore, and Structure Configuration.
   - Manages UI session state (open tabs, active navigation stack, scroll/cursor positions, panel toggle states).
   - Contains zero domain rules; delegates all structural modifications to the Application Layer via IPC commands.

2. **IPC Bridge (Tauri Command Interface):**
   - Strong type-safe serialization boundary between Web View and Rust Core using `specta` / `tauri-specta`.
   - Converts UI actions into strongly typed Rust domain commands (e.g., `CreateEntryCommand`, `ReparentSpatialCommand`, `SaveManuscriptCommand`).

3. **Application / Use Case Layer (Rust):**
   - Orchestrates domain workflows, input validation, transaction boundaries, and event notifications.
   - Coordinates persistence writes, search index updates, and backup triggers.

4. **Domain Model Engine (Rust):**
   - Framework-agnostic pure Rust domain entities and business rules.
   - Enforces hard invariants: Spatial graph acyclicity, single-source relationship projections, merged field availability, capability composition, template evolution rules.

5. **Persistence & Integrity Layer (Rust + SQLite):**
   - Encapsulates database connection pool, WAL mode configuration, repository traits, and SQL execution.
   - Executes structural changes inside SQLite transactions (`BEGIN IMMEDIATE ... COMMIT`). Handles schema migrations via embedded `refinery`.

6. **Search & Indexing Engine (Rust + SQLite FTS5):**
   - Maintains the FTS5 virtual table for full-text search.
   - Executes recursive SQLite CTE queries for spatial ancestry and derived backlinks.

7. **Backup & Recovery Engine (Rust):**
   - Runs background rolling backups, creates ZIP snapshots of the `.wcproj` package, and handles emergency state persistence on system signals.

---

## 3. Domain Model

All identifiable entities inherit stable identity via 128-bit time-sortable **UUIDv7** strings.

```
                         ┌─────────────┐
                         │   Project   │
                         └──────┬──────┘
                                │ 1:N
         ┌──────────────────────┼──────────────────────┐
         ▼                      ▼                      ▼
  ┌─────────────┐        ┌─────────────┐        ┌─────────────┐
  │  Category   │        │    Entry    │        │ Story Unit  │
  └──────┬──────┘        └──────┬──────┘        │  (Chapter)  │
         │ 1:N                  │               └──────┬──────┘
         ▼                      │                      │
  ┌─────────────┐               │                      │ 1:N
  │    Type     │               │                      ▼
  └─────────────┘               │             ┌─────────────────┐
                                │             │ Chapter-Entry   │
         ┌──────────────────────┼────────────>│     Link        │
         │                      │             └─────────────────┘
         ▼                      ▼
  ┌─────────────┐        ┌─────────────┐
  │ Capability  │        │ SpatialData │ (1:0..1)
  │ (Junction)  │        └─────────────┘
  └─────────────┘
         │
         ▼
  ┌─────────────┐        ┌─────────────┐        ┌──────────────────┐
  │    Field    │        │Relationship │        │   Relationship   │
  │ Definition  │        │ Definition  │        │     Instance     │
  └──────┬──────┘        └──────┬──────┘        └────────┬─────────┘
         │ 1:N                  │ 1:N                    │
         ▼                      └────────────────────────┤ (Shared Fact)
  ┌─────────────┐                                        │
  │ Field Value │◄───────────────────────────────────────┘
  └─────────────┘  (Relationship-Backed Field Projection)
```

### Primary Entity Definitions

#### Project
- `id`: `UUIDv7` (Primary Key)
- `name`: `String`
- `created_at`: `DateTime<Utc>`
- `updated_at`: `DateTime<Utc>`
- `schema_version`: `u32`

#### Entry
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7` (Foreign Key)
- `category_id`: `UUIDv7` (Foreign Key to Category; fallback to Uncategorized)
- `type_id`: `Option<UUIDv7>` (Optional Foreign Key to Type)
- `display_name`: `String` (Author-visible label; editable without breaking references)
- `description_json`: `Option<String>` (ProseMirror JSON document)
- `archived_at`: `Option<DateTime<Utc>>`
- `trashed_at`: `Option<DateTime<Utc>>`
- `created_at`: `DateTime<Utc>`
- `updated_at`: `DateTime<Utc>`

#### Category
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7`
- `name`: `String` (e.g., Characters, Objects, Creatures, Places)
- `description`: `Option<String>`
- `is_uncategorized_fallback`: `bool`
- `sort_order`: `i32`

#### Type
- `id`: `UUIDv7` (Primary Key)
- `category_id`: `UUIDv7` (Foreign Key)
- `name`: `String` (e.g., Human, Weapon, World Turtle, City)
- `parent_type_id`: `Option<UUIDv7>`
- `is_retired`: `bool`

#### Field Definition
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7`
- `owner_kind`: `FieldOwnerKind` (`Category` | `Type` | `Entry`)
- `owner_id`: `UUIDv7`
- `name`: `String`
- `help_text`: `Option<String>`
- `value_kind`: `FieldValueKind` (`ShortText`, `RichText`, `Number`, `Choice`, `MultiChoice`, `Boolean`, `EntryReference`)
- `cardinality`: `FieldCardinality` (`Single`, `Multiple`)
- `relationship_definition_id`: `Option<UUIDv7>` (Set if field is relationship-backed)
- `allowed_choices_json`: `Option<String>`
- `is_retired`: `bool`
- `sort_order`: `i32`

#### Field Value
- `id`: `UUIDv7` (Primary Key)
- `entry_id`: `UUIDv7` (Foreign Key)
- `field_definition_id`: `UUIDv7` (Foreign Key)
- `value_json`: `String` (JSON serialization of value)

#### Relationship Definition
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7`
- `name`: `String` (e.g., Ownership)
- `forward_label`: `String` (e.g., "owns")
- `inverse_label`: `String` (e.g., "is owned by")
- `is_symmetric`: `bool`
- `is_retired`: `bool`

#### Relationship Instance
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7`
- `relationship_definition_id`: `UUIDv7` (Foreign Key)
- `source_id`: `UUIDv7` (Foreign Key to Entry/Record A)
- `target_id`: `UUIDv7` (Foreign Key to Entry/Record B)
- `note`: `Option<String>`
- `created_at`: `DateTime<Utc>`
- `updated_at`: `DateTime<Utc>`

#### Capability (Entry Capability Junction)
- `entry_id`: `UUIDv7` (Foreign Key)
- `capability_id`: `String` (`"base"`, `"spatial"`)

#### Spatial Data
- `entry_id`: `UUIDv7` (Primary Key & Foreign Key to Entry)
- `primary_parent_entry_id`: `Option<UUIDv7>` (Foreign Key to Spatial Entry)

#### Story Unit (Chapter)
- `id`: `UUIDv7` (Primary Key)
- `project_id`: `UUIDv7`
- `unit_type`: `String` (`"chapter"`)
- `title`: `String`
- `sort_order`: `i32`
- `manuscript_json`: `Option<String>` (ProseMirror AST JSON)
- `notes_json`: `Option<String>`
- `word_count`: `u32`
- `archived_at`: `Option<DateTime<Utc>>`
- `trashed_at`: `Option<DateTime<Utc>>`

#### Story Usage / Chapter Link
- `id`: `UUIDv7` (Primary Key)
- `chapter_id`: `UUIDv7` (Foreign Key)
- `entry_id`: `UUIDv7` (Foreign Key)
- `role`: `StoryRole` (`"pov"`, `"setting"`, `"appears"`, `"mentioned"`)

---

## 4. Relationship Architecture

The relationship system strictly enforces **Single Source of Truth** for semantic connections.

### Single Instance, Dual Projection
A semantic relationship is persisted as exactly **one** row in the `relationship_instance` table.

Canonical Example: `Thron owns Singularity Blade`
- Row in `relationship_instance`:
  - `id`: `rel_instance_101`
  - `relationship_definition_id`: `rel_def_ownership` (`forward_label = "owns"`, `inverse_label = "is owned by"`)
  - `source_id`: `thron_id`
  - `target_id`: `blade_id`

#### Projections:
- **Forward View (on Thron):** Resolves connections where `source_id = thron_id`. Renders: `owns → Singularity Blade`.
- **Inverse View (on Singularity Blade):** Resolves connections where `target_id = blade_id`. Renders: `is owned by → Thron`.

Editing the owner on the Blade editor updates `rel_instance_101`. Thron's view instantly reflects the update because both views query the same row.

### Relationship-Backed Fields
Fields such as `Current Owner` or `Birthplace` are projected editors over `relationship_instance`.
When a Field Definition has `relationship_definition_id` set:
- Reading the field executes a query against `relationship_instance`.
- Updating the field executes an `UPSERT` or `UPDATE` on `relationship_instance`.
- **No duplicate data** is written to `field_value`.

### De-duplication in UI
When rendering the Entry editor:
- The UI retrieves relationship-backed fields (e.g. `Current Owner: Thron`).
- The general "Relationships" section filters out any relationship instance ID already displayed by a relationship-backed field on that entry, avoiding duplicate visual representation.

### Extensibility to Rich Knots
Every `relationship_instance` has a stable `UUIDv7` identity. Adding notes, acquisition dates, or custom attributes modifies attributes on that instance. 
Future multi-party relationships (e.g., alliances, battles) can be introduced seamlessly by adding an auxiliary `relationship_participant(instance_id, record_id, role)` table without replacing the binary relationship schema.

---

## 5. Field / Template Architecture

### Merged Field Availability
An Entry's available fields are computed dynamically by taking the union of:
1. Field Definitions owned by the Entry's Category.
2. Field Definitions owned by the Entry's Type (if assigned).
3. Field Definitions created locally for this specific Entry.

```
Merged Available Fields = Category.Fields ∪ Type.Fields ∪ Entry.LocalFields
```

### Empty Inherited Fields
A template makes a field *available*, but does **not** insert an empty row in `field_value`.
- If `Human` defines `Eye color`, but `Thron` has not authored an eye color, the UI displays an empty input for `Eye color`.
- Database storage remains zero rows for `Thron`'s `Eye color` until a value is explicitly entered.

### Local Field Promotion
When an author creates a local field (e.g. `Shell diameter` on `Tortuga`) and later selects "Make available to Type: World Turtle":
1. The `FieldDefinition` row's `owner_kind` is updated from `Entry` to `Type`, and `owner_id` is updated to `world_turtle_type_id`.
2. `FieldDefinition.id` remains unchanged.
3. Existing `FieldValue` rows maintain their `field_definition_id` references without re-entry.
4. Other `World Turtle` entries immediately gain the field as available.

### Retired Definitions
When a field is removed from a Type or Category template:
1. `FieldDefinition.is_retired` is set to `true` (or the template junction is detached).
2. Existing `FieldValue` rows on historical entries **survive intact**.
3. Historical entries render the field with a "Retired Field" indicator.
4. New entries of that Type/Category no longer present the field in their available template.

### Safe Category and Type Changes
Changing an Entry's Type or Category updates `entry.type_id` or `entry.category_id`.
- All populated `FieldValue` rows survive because they reference `field_definition_id` directly.
- Template fields no longer relevant become detached historical values rather than being deleted.

---

## 6. Capability Architecture

Capabilities separate **classification** (Category/Type) from **specialized system behavior**.

```
┌─────────────────────────────────────────────────────────────┐
│                          TORTUGA                            │
│  Category: Creatures (Organizational Filing)               │
│  Type: World Turtle (Template / Classification)             │
├─────────────────────────────────────────────────────────────┤
│                       CAPABILITIES                          │
│  [x] BaseCapability     (Description, Fields, Relations)    │
│  [x] SpatialCapability  (Hierarchy, Parent, Children)      │
└─────────────────────────────────────────────────────────────┘
```

### System-Defined Capabilities in Milestone 01
1. **BaseCapability:** Attached to every Entry automatically. Provides display name, description, fields, relationships, tags, and story links.
2. **SpatialCapability:** Provides spatial parent containment, spatial child listing, breadcrumb derivation, and location relationship integration.

### Execution Model
- Capabilities are system-defined and managed via the `entry_capability` junction table.
- Categories and Types can specify default capabilities (e.g. Category `Places` defaults to `Base + Spatial`).
- Individual Entries can add or remove compatible capabilities dynamically.
- Adding `SpatialCapability` to `Tortuga` inserts `(tortuga_id, 'spatial')` and a record in `spatial_data`. Tortuga's Category remains `Creatures` and its Type remains `World Turtle`.

---

## 7. Spatial Architecture

Spatial structure models navigable fictional space without forcing spatial entries into the `Places` Category.

### Primary Hierarchy and Containment
- Primary spatial containment is stored in `spatial_data(entry_id, primary_parent_entry_id)`.
- A Spatial Entry has **zero or one** primary spatial parent, and **zero or more** direct spatial children.
- Root Spatial Entries (`primary_parent_entry_id = NULL`) are fully supported.

### Cycle Prevention Algorithm
Before executing `SET primary_parent_entry_id = P` for Entry `E`:
1. Execute an upward recursive query starting from `P`.
2. If `E` is encountered anywhere in `P`'s ancestry chain, abort the transaction and return a `SpatialCycleException`.
3. The UI blocks cyclic drag-and-drop assignments immediately.

### Structural Containment vs Current Location
- **Structural Containment:** `Temple of the First Step` is inside `City of Arak` (`spatial_data` hierarchy).
- **Current / Situational Location:** `Thron` is currently in `Temple of the First Step` (stored in `relationship_instance` with `RelationshipDefinition = "current_location"`).

### Derived Recursive Ancestry
When resolving the full location context for `Thron`:
1. Fetch direct location relationship: `Thron → Current Location → Temple of the First Step`.
2. Execute SQLite Recursive CTE query upward from `Temple of the First Step`:

```sql
WITH RECURSIVE SpatialAncestry AS (
    SELECT entry_id, primary_parent_entry_id, 0 AS depth
    FROM spatial_data
    WHERE entry_id = 'temple_id'
    
    UNION ALL
    
    SELECT s.entry_id, s.primary_parent_entry_id, sa.depth + 1
    FROM spatial_data s
    INNER JOIN SpatialAncestry sa ON s.entry_id = sa.primary_parent_entry_id
)
SELECT e.id, e.display_name
FROM SpatialAncestry sa
JOIN entry e ON e.id = sa.entry_id;
```

3. Results yield derived ancestry: `Temple of the First Step → City of Arak → Northern Shell → Tortuga`.
4. **No duplicate facts** are stored for Arak, Northern Shell, or Tortuga on Thron's record.

### Reparenting Behavior
When `City of Arak` is reparented from `Northern Shell` to `Floating Continent`:
- Update `spatial_data.primary_parent_entry_id` for `Arak`.
- `Arak`, `Temple`, and `Thron` keep their exact UUIDv7 IDs.
- `Temple`'s derived breadcrumb automatically updates to: `Floating Continent > City of Arak > Temple of the First Step`.
- `Thron`'s derived location context updates automatically.

---

## 8. Story / Chapter Architecture

Story Units represent narrative prose and reading structure, remaining strictly separate from world Entries.

### Chapter Model
Milestone 01 implements loose **Chapters**:
- `story_unit` table with `unit_type = 'chapter'`.
- Properties: `id`, `title`, `sort_order`, `manuscript_json` (ProseMirror JSON), `notes_json`, `word_count`.
- Loose chapters do not require a parent Book or Part.

### Bidirectional Chapter-Entry Links
Chapter links are stored in `chapter_entry_link(chapter_id, entry_id, role)`.
- Roles: `POV`, `Setting`, `Appears`, `Mentioned`.
- **Chapter View:** Queries links for `chapter_id` to show linked entities in the right-hand context panel.
- **Entry View:** Queries links for `entry_id` to display the `Story Usage` backlink section.

### Derived Spatial Story Context
When Chapter `The First Step` links `Temple of the First Step` as `Setting`:
- The manuscript context panel resolves `Temple`'s spatial ancestry.
- Context panel displays: `Setting: Temple of the First Step (within Tortuga > Northern Shell > City of Arak)`.
- Ancestors are highlighted as derived setting context, not independent manual chapter links.

### Extension Path for Future Story Units
Future structures (`Book`, `Part`, `Scene`) can be added by adding `parent_unit_id` to `story_unit` without altering Chapter link schemas or manuscript storage.

---

## 9. Persistence Model

Worldcrafter uses a **local-first, self-contained Project bundle format**.

### Package Layout on Disk
A Worldcrafter Project is stored as a directory package named `<ProjectName>.wcproj/`:
```
MyStory.wcproj/
├── project.sqlite        # SQLite 3 Database (WAL mode enabled)
├── project.sqlite-wal    # Write-Ahead Log
├── project.sqlite-shm    # Shared Memory File
├── assets/               # Managed attachments and images
└── backups/              # Rolling automatic backup archives
```

### SQLite Configuration & Transaction Safety
- `PRAGMA journal_mode = WAL;` (Ensures concurrent reads/writes and crash resilience).
- `PRAGMA synchronous = NORMAL;` (Provides durability with optimal disk write performance).
- `PRAGMA foreign_keys = ON;` (Enforces relational integrity).
- All multi-table operations execute within explicit Rust-managed SQLite transactions:
  ```rust
  let tx = pool.begin().await?;
  // Execute domain operations
  tx.commit().await?;
  ```

### Schema Versioning & Migrations
- Managed via `refinery` migration scripts embedded in the Rust binary.
- On opening a Project, schema version in `project.sqlite` is verified. Upgrades run sequentially inside isolated transactions.

---

## 10. Autosave and Crash Safety

The status **`Saved`** guarantees that authored data has been durably written to disk via an explicit SQLite transaction commit.

### Autosave Strategy

```
User Types in Manuscript
          │
          ▼
   Debounce (300ms)
          │
          ▼
   Tauri IPC Command: save_manuscript(chapter_id, prose_json)
          │
          ▼
   Rust Persistence: BEGIN TRANSACTION -> UPDATE story_unit -> COMMIT
          │
          ▼
   IPC Response: Ok(Saved)
          │
          ▼
   UI Updates Status Indicator: "Saved"
```

- **Manuscript Autosave:** Debounced at 300ms of typing idle time; flushes to SQLite. On window blur or tab switch, flush is immediate.
- **Structural Changes:** Reparenting, relationship edits, field changes, and renames bypass debouncing and save synchronously to SQLite.

### Crash Safety & Recovery
- Because WAL mode is enabled, any committed transaction is immediately crash-safe.
- If process termination occurs mid-edit, uncommitted transactions rollback cleanly. Upon restart, SQLite performs WAL recovery automatically.
- On abnormal termination, a subtle notification informs the user: *"Project recovered cleanly to last saved state."*

---

## 11. Backup / Recovery Design

Backup, Trash, Archive, and Undo are kept conceptually separate:

| Mechanism | Scope & Purpose | Recovery Method |
| :--- | :--- | :--- |
| **Undo** | In-memory stack for immediate editing mistakes in active session | `Ctrl+Z` / `Cmd+Z` |
| **Archive** | Soft-hide inactive records from workspace without breaking links | Toggle "Show Archived" / Unarchive action |
| **Trash** | Recoverable soft-deletion for records | Open Trash view -> Restore record |
| **Backup** | Full-project snapshot protecting against disk corruption or accidental deletion | Restore Backup as Copy |

### Backup Implementation
- **Rolling Local Backups:** Triggered every 30 minutes of active use and on application shutdown. Saved to `~/.config/worldcrafter/backups/<Project_ID>/`. Retains last 10 hourly and 7 daily snapshots.
- **Manual Snapshot:** User can trigger "Create Backup Now" to produce a compressed `.wcpack` bundle containing `project.sqlite` and `assets/`.
- **Restore as Copy:** Restoring a backup creates a new `.wcproj` directory with a newly generated `UUIDv7` Project ID, preventing overwriting current work.

---

## 12. Search and Backlinks

### Embedded Search Engine
Search is powered by an SQLite **FTS5** virtual table (`search_index`).

```sql
CREATE VIRTUAL TABLE search_index USING fts5(
    record_id,
    record_kind, -- 'entry', 'story_unit', 'field', 'manuscript'
    display_name,
    alias_text,
    content_text,
    tokenize = 'porter unicode61'
);
```

### Ranking Hierarchy
Search results are scored and ordered:
1. **Exact Display Name / ID Match** (Score 100)
2. **Alias Match** (Score 90)
3. **Structured Field / Tag Match** (Score 70)
4. **Description / Manuscript Text Match** (Score 40)

Searching `Thron` prioritizes the `Thron` Character Entry above hundreds of manuscript prose text matches.

### Result Grouping
UI presents search results categorized into:
- **Entries** (World entities)
- **Story Units** (Chapters)
- **Structured Fields**
- **Manuscript Text**

### Rebuildable Index Architecture
`search_index` is a derived cache. Rebuilding the index drops `search_index` and repopulates it from `entry`, `field_value`, and `story_unit` tables.

---

## 13. Navigation / UI State Architecture

UI navigation state is decoupled from domain persistence to enable fast multi-tab browsing without mutating project data.

```
┌─────────────────────────────────────────────────────────────┐
│                      NAV CONTAINER                          │
│  [Tab 1: Thron (Active)] [Tab 2: Temple] [Tab 3: Chapter 1] │
├─────────────────────────────────────────────────────────────┤
│  HISTORY STACK (Tab 1):                                     │
│  1. Entry: Thron  <-- Current Pointer                       │
│  2. Entry: Singularity Blade                                │
│  3. Spatial: Tortuga                                        │
└─────────────────────────────────────────────────────────────┘
```

### Navigation Stack Components
- **Multi-Tab State:** Array of active tab instances managed in React (`Zustand` store).
- **Per-Tab History Stack:** Standard `Back` and `Forward` stack containing `(record_type, record_id, scroll_top, cursor_position)`.
- **Context Preview Cards:** Hovering or clicking a referenced record opens an inline popover context card without clearing active navigation history.
- **Cursor & Scroll Preservation:** Switching tabs or opening side context panels preserves ProseMirror editor cursor selections and scroll offsets.

---

## 14. Deletion and Lifecycle Strategy

Lifecycle states dictate record visibility and reference integrity:

```
   [Active Record]
          │
     ┌────┴───────────────────────────┐
     ▼                                ▼
[Archived State]               [Trashed State]
(Hidden from default lists,     (Soft deleted, recoverable,
 references stay intact)        references stay intact)
                                      │
                                      ▼
                           [Permanent Deletion]
                           (Hard delete from DB)
                                      │
                                      ▼
                        [Unresolved Reference Created]
                        (Stores last-known display name)
```

### Deletion Rules
1. **Archive:** Sets `archived_at`. Record remains fully resolvable in relationships, backlinks, and spatial trees with an `[Archived]` tag.
2. **Trash:** Sets `trashed_at`. Record is hidden from active searches. Relationships remain attached so restoring recovers full context.
3. **Permanent Deletion of Spatial Parent:** Permanent deletion of a Spatial Parent requires explicit user choice:
   - *Option A:* Reparent direct children to grandparent.
   - *Option B:* Convert direct children to Root Spatial Entries.
   - Permanent deletion never silently cascades through descendant subtrees.
4. **Unresolved Reference Handling:** If a relationship target is permanently deleted, `relationship_instance` is converted to an `unresolved_reference` record storing `target_id` and `last_known_display_name`. UI renders the link with a broken icon and repair action.

---

## 15. Project / Package Portability

To guarantee that Projects remain portable across devices, operating systems, and directory moves:
1. **No Absolute Filesystem Paths:** All internal references use relative bundle paths (e.g. `assets/images/sword.png`) or UUIDv7 identifiers.
2. **Self-Contained Bundle:** Moving a `.wcproj` directory to a USB drive or different operating system preserves every link, asset, and relation.
3. **Cross-Platform SQLite:** SQLite database files are binary portable between Windows, macOS, and Linux.

---

## 16. Testing Strategy

We define a three-tiered automated testing pyramid focusing heavily on domain invariants:

```
           /\
          /  \     E2E Acceptance Tests (Playwright / Tauri Driver)
         /    \    Canonical Scenario, Tab Nav, Crash Recovery
        /------\
       /        \   Frontend Component Tests (Vitest + React Testing Library)
      /          \  Editor extensions, UI State, Panel Toggles
     /------------\
    /              \  Rust Unit & Integration Tests (Cargo Test)
   /                \ Domain Invariants, UUIDs, Spatial CTEs,
  /------------------\ SQLite Migrations, FTS5 Search Ranking, Transactions
```

### Key Domain Invariant Test Coverage (Rust)
- **Stable Identity:** Renaming records preserves UUIDv7 references across fields, relationships, spatial paths, and chapter links.
- **Single Relationship Fact:** Assert forward and inverse views resolve from a single `relationship_instance` row.
- **Spatial Cycle Prevention:** Assert attempting to parent an ancestor under a descendant raises `SpatialCycleException`.
- **Derived Spatial Ancestry:** Assert reparenting `Arak` updates `Temple` and `Thron` derived paths without mutating `Temple` or `Thron` rows.
- **Template Evolution:** Assert retiring a field definition preserves historical authored `FieldValue` rows.
- **Persistence & WAL Atomicity:** Assert process kills during multi-table writes leave SQLite database in a clean, consistent pre- or post-state.

---

## 17. Proposed Implementation Phases

We propose the following 9 phased implementation steps to ensure vertical, testable deliverables at each stage:

| Phase | Title | Major Components | Demonstrable Output |
| :--- | :--- | :--- | :--- |
| **Phase 0** | Technical Foundation | Tauri 2.0 app skeleton, Rust engine, SQLite WAL schema, UUIDv7 generator, `refinery` migration engine, transaction manager. | App opens, creates empty `.wcproj` file on disk, runs migrations, passes Rust invariant tests. |
| **Phase 1** | Project & Entry Core | Project creation, Project Home, Category/Type management, Entry editor (display name, description), basic navigation. | Create project `Thron`, create Category `Characters`, Type `Human`, Entry `Thron`. Reopen app to confirm state survives. |
| **Phase 2** | Field Architecture | Field Definitions, Field Values, merged template availability, local field creation, local promotion, retired field retention. | Add local field to `Tortuga`, promote to Type `World Turtle`. Add and retire fields safely. |
| **Phase 3** | Relationship Engine | Relationship Definitions, Relationship Instances, forward/inverse projections, relationship-backed fields, quick-create targets. | Create `Thron owns Singularity Blade` via quick-create. Verify dual projection on both Entry editors. |
| **Phase 4** | Spatial Capability | Spatial Capability, primary parent assignment, cycle prevention, reparenting engine, location relationships, derived ancestry CTEs. | Build `Tortuga > Northern Shell > Arak > Temple`. Set `Thron Location = Temple`. Reparent `Arak` and verify derived paths. |
| **Phase 5** | Search & Navigation | SQLite FTS5 indexer, BM25 ranking, backlinks engine, Explore filters, multi-tab state, Recent/Pinned lists. | Search `Thron` and verify Entry ranks top. Navigate via tabs and backlinks seamlessly. |
| **Phase 6** | Chapter & Manuscript | Loose Chapter editor, ProseMirror integration, Chapter-Entry links (`POV`, `Setting`), collapsible context panel, Story usage. | Write Chapter `The First Step` with `Thron (POV)` and `Temple (Setting)`. Context panel displays derived spatial setting. |
| **Phase 7** | Safety & Backups | Archive, Trash, Restore, unresolved reference repair, rolling backup engine, crash recovery handler. | Archive/Trash `Blade` and restore. Trigger manual backup snapshot and restore as copy. |
| **Phase 8** | Integration Torture | Canonical E2E acceptance test suite, rename torture test, spatial reparent torture test, crash simulation. | Run full automated acceptance suite verifying all 17 canonical steps. |

---

## 18. Risks and Difficult Decisions

### Risk 1: ProseMirror / TipTap Editor Sync with SQLite Persistence
- **Risk:** High-frequency manuscript typing could create IPC bottlenecks or cursor jumps if UI re-renders on backend persistence returns.
- **Mitigation:** Decouple local React TipTap state from Rust persistence calls. Debounce SQLite writes at 300ms. Send unidirectional IPC updates with pending dirty flags.

### Risk 2: Performance of Derived Spatial Ancestry CTEs
- **Risk:** Deep spatial trees could slow down search or context panel rendering.
- **Mitigation:** SQLite CTE queries over indexed `spatial_data(entry_id, primary_parent_entry_id)` execute in sub-millisecond times (<0.5ms for trees of 10,000+ nodes). Benchmarks will be enforced in Cargo performance tests.

### Risk 3: Structural Integrity During Unclean Application Shutdown
- **Risk:** System crash during multi-table reparenting or template migration could corrupt project data.
- **Mitigation:** Enforce SQLite WAL mode and wrap all structural mutations inside Rust transaction blocks. SQLite guarantees atomic rollback on next database initialization.

---

## Product Questions / Decisions Required Before Implementation

### Question 1: Category Reassignment when Deleting a Category
- **Unresolved Issue:** When deleting a Category that contains active Entries, what is the default target Category?
- **Why Architecture Depends On It:** Dictates whether Category deletion requires an interactive selection UI or automatic fallback.
- **Options:**
  - *Option A:* Mandatory prompt asking author to select an existing destination Category.
  - *Option B:* Automatically reassign all Entries in the deleted Category to the system `Uncategorized` fallback.
- **Technical Recommendation:** Option B with an optional prompt if multiple entries exist. Implementation will default to reassigning to `Uncategorized` to guarantee entries are never deleted.
- **Can Implementation Proceed?** Yes.

### Question 2: Cascade Policy for Children of Trashed Spatial Parents
- **Unresolved Issue:** When a Spatial Parent (e.g. `City of Arak`) is moved to **Trash** (soft deleted), should its direct spatial children (`Temple of the First Step`) also move to Trash, or remain active with a reference to a trashed parent?
- **Why Architecture Depends On It:** Affects soft-delete cascading logic in the Spatial engine.
- **Options:**
  - *Option A:* Children remain in active workspace; their primary parent pointer continues pointing to `Arak` (displayed with a `[Trashed Parent]` badge in breadcrumbs).
  - *Option B:* Soft-delete cascades to all direct and indirect spatial children.
- **Technical Recommendation:** Option A. Soft-deleting a parent should not hide children without explicit user confirmation. Restoring `Arak` immediately restores intact hierarchy.
- **Can Implementation Proceed?** Yes.

### Question 3: Passage-Level Manuscript Mentions vs Renaming
- **Unresolved Issue:** Should Milestone 01 include passage-level inline text mention decorations in ProseMirror, or stick strictly to Chapter-level links?
- **Why Architecture Depends On It:** Passage-level mentions require custom ProseMirror mark extensions.
- **Options:**
  - *Option A:* Include passage-level mention marks in Milestone 01.
  - *Option B:* Defer passage-level marks to Milestone 02; implement Chapter-level record links and context panel for Milestone 01.
- **Technical Recommendation:** Option B. Chapter-level links fulfill all Milestone 01 acceptance criteria without adding ProseMirror mark schema complexity.
- **Can Implementation Proceed?** Yes.

---

## Canonical Architecture Scenario Verification

We verify that the proposed architecture successfully resolves every step of the canonical test scenario:

1. **Create Project `Thron`:** Creates `Thron.wcproj/` directory, initializes `project.sqlite` with WAL mode, runs migrations, sets Project ID.
2. **Create Character `Thron`:** Inserts row into `entry` with Category `Characters` and Type `Human`.
3. **Quick-create `Singularity Blade` via Ownership:** Creates Object Entry `Singularity Blade` and inserts a single row into `relationship_instance` (`Thron owns Singularity Blade`). Dual projections render on both Entry editors.
4. **Create `Tortuga` (Category: Creatures, Type: World Turtle):** Inserts Entry row.
5. **Add Spatial Capability to Tortuga:** Inserts row into `entry_capability` (`tortuga_id`, `'spatial'`) and `spatial_data` (`tortuga_id`, `NULL`). Category remains `Creatures`.
6. **Create Spatial Hierarchy:**
   - `Northern Shell` (`primary_parent = Tortuga`)
   - `City of Arak` (`primary_parent = Northern Shell`)
   - `Temple of the First Step` (`primary_parent = City of Arak`)
7. **Set `Thron Current Location = Temple`:** Inserts single row into `relationship_instance` (`Thron → current_location → Temple`). Derived CTE query calculates derived ancestors (`Arak > Northern Shell > Tortuga`).
8. **Navigation:** User navigates Thron → Blade → Temple → Arak → Tortuga. History stack preserves tab scroll state and back button history.
9. **Create Chapter `The First Step`:** Inserts row in `story_unit`.
10. **Link Thron as POV:** Inserts row in `chapter_entry_link` (`role = 'pov'`).
11. **Link Temple as Setting:** Inserts row in `chapter_entry_link` (`role = 'setting'`). Context panel resolves derived spatial ancestry for Temple.
12. **Link Singularity Blade:** Inserts row in `chapter_entry_link`.
13. **Write Manuscript Prose:** ProseMirror AST saved to `story_unit.manuscript_json` via debounced transaction.
14. **Rename Thron and Tortuga:** Updates `display_name` on Entry rows. Stable UUIDv7 IDs ensure all relationships, spatial paths, and chapter links remain valid. Manuscript text is preserved as authored.
15. **Reparent Arak:** Updates `spatial_data.primary_parent_entry_id` for Arak to `Floating Continent`. Derived CTE queries automatically project updated breadcrumbs for Temple and Thron.
16. **Archive/Restore Blade:** Sets `archived_at` on Blade. References show `[Archived]` badge. Restoring clears `archived_at` with all connections intact.
17. **Close and Reopen:** SQLite WAL mode guarantees 100% data preservation and session restoration.
