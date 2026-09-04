# The Worldcrafter

**The Worldcrafter** is a local-first desktop application for worldbuilding and story writing.

Its goal is to let authors create fictional material naturally, connect it where useful, and write stories without forcing every idea into a rigid predefined structure.

Worldbuilding and writing are both first-class parts of the application, but neither depends on the other.

An author should be able to start with a character, an object, a place, a chapter, or almost nothing at all — and add structure only when it becomes useful.

---

## Current Status

The project is currently in the **architecture and first implementation milestone** stage.

The product concept for the current version has been defined, and Milestone 01 specifies the first real vertical slice of the application.

The implementation should not begin by trying to build the entire long-term Worldcrafter vision.

The first goal is to prove the core loop:

> Create material → describe it → connect it → navigate it → use it while writing → safely change it later.

---

## Documentation

### Product Concept

`docs/concept/The_Worldcrafter_Concept_V0.02.docx`

This is the main product and conceptual source of truth.

It defines:

* product philosophy
* Projects
* Entries
* Categories
* Types
* Fields
* Relationships
* Capabilities
* Spatial structure
* Story Units
* search and backlinks
* navigation
* writing workflows
* deletion and recovery behavior
* persistence and backups
* long-term architectural direction
* deferred systems

### Milestone 01

`docs/milestones/MILESTONE_01_IMPLEMENTATION.md`

This translates the product concept into the engineering contract for the first implementation milestone.

It contains:

* hard invariants
* required behavior
* Milestone 01 scope
* explicit non-goals
* canonical integration data
* acceptance tests
* torture tests
* architecture review requirements
* implementation guidance

---

## Milestone 01

Milestone 01 is intended to produce the first version of Worldcrafter that proves the architecture and central creative workflow.

The canonical integration scenario includes:

* **Thron** — Character
* **Singularity Blade** — Object related to Thron
* **Tortuga** — Creature with Spatial capability
* **Northern Shell**
* **City of Arak**
* **Temple of the First Step**
* **The First Step** — Chapter using the world material

The application should allow these records to be created, connected, searched, navigated, reorganized, written with, renamed, archived, restored, saved, closed, and reopened without breaking their identities or relationships.

---

## Core Product Principles

### Start anywhere

The application must not require the author to fill out an encyclopedia before they can work.

Types, structured Fields, Spatial hierarchy, Story planning, and other systems are optional tools.

### Stable identity

Visible names are not identifiers.

Renaming a character, place, Category, Type, Field, or relationship must not break references.

### One source of truth

Semantic relationships should be stored once and projected wherever they are useful.

For example:

`Thron owns Singularity Blade`

and:

`Singularity Blade is owned by Thron`

must represent the same underlying relationship.

### Preserve authored data

Changing templates, Categories, Types, or definitions must not silently delete information the author has already entered.

### Local-first

Core Worldcrafter functionality must work without an account, cloud service, or internet connection.

### Preserve creative momentum

The system should allow missing Categories, Types, Entries, and relationships to be created inline without forcing the author away from their current task.

### Reversible by default

Renaming, moving, reparenting, archiving, and ordinary restructuring should be safe and reversible.

Permanent destructive operations should be exceptional.

---

## Development Process

The current development process is:

1. Define product behavior and workflows.
2. Record those decisions in the Concept document.
3. Translate the relevant slice into a Milestone implementation specification.
4. Produce and review a technical architecture proposal.
5. Implement the milestone incrementally.
6. Test the actual creative workflows.
7. Feed implementation discoveries back into the product concept.

The first engineering task is **not** to implement Milestone 01 immediately.

The first task is to produce:

`docs/architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL.md`

The architecture proposal must be reviewed before major implementation begins.

---

## Scope Discipline

The Worldcrafter concept intentionally includes systems that are **not part of Milestone 01**.

Examples include:

* advanced timelines
* fictional calendars
* editable maps
* continuity checking
* graph visualization
* storyboards
* advanced Book / Part / Scene structures
* full multi-party relationship knots
* historical state evaluation
* collaboration
* cloud sync
* mobile support
* AI-assisted worldbuilding

These ideas may influence architecture, but they should not be implemented unless explicitly brought into scope.

---

## Repository Structure

The intended documentation structure is:

```text
docs/
├── architecture/
├── concept/
│   └── The_Worldcrafter_Concept_V0.02.docx
└── milestones/
    └── MILESTONE_01_IMPLEMENTATION.md
```

Additional source, test, build, and tooling structure should be proposed as part of the architecture phase.

---

## Current Next Step

Read:

1. `docs/concept/The_Worldcrafter_Concept_V0.02.docx`
2. `docs/milestones/MILESTONE_01_IMPLEMENTATION.md`

Then inspect the repository and produce:

`docs/architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL.md`

Do not begin broad application implementation before the architecture proposal has been reviewed.
## The Worldcrafter application

The Worldcrafter is a local-first desktop application for worldbuilding and story
writing. Product concept and scope are defined in `docs/concept/` and
`docs/milestones/`; approved technical architecture is in `docs/architecture/`.

The application lives in `app/` (Tauri 2 + React 19 + TypeScript frontend, Rust
backend in `app/src-tauri`).

### Prerequisites

* Node.js 20+ and npm
* Rust stable (via `rustup`)
* Linux only: Tauri's native build dependencies, e.g. on Debian/Ubuntu:
  `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev build-essential libssl-dev`

### Install

```sh
cd app
npm install
```

### Development

```sh
cd app
npm run tauri dev   # full desktop app (requires a display/windowing environment)
npm run dev         # frontend only, in a browser, against a mock/no backend
```

### Tests

```sh
cd app
npm test                          # frontend unit tests (vitest)
cd src-tauri && cargo test        # Rust unit/integration tests
```

### Checks / build

```sh
cd app
npm run typecheck                 # TypeScript --noEmit
npm run lint                      # eslint
npm run format:check              # prettier --check
npm run build                     # production frontend build (tsc + vite build)
cd src-tauri
cargo fmt --check
cargo clippy --all-targets
cargo check --bins                # compiles the Tauri binary target
cargo tauri build                 # full desktop bundle; requires a display/windowing
                                   # environment and platform packaging tools, and is
                                   # not expected to succeed in a headless CI/sandbox
```

### Current implemented slice: Project Structure Backbone (Milestone 01, Task 02A)

This slice proves the desktop stack starts, a real self-contained `.wcproj`
package can be created/opened/renamed/closed, Project identity (UUIDv7) is
stable and independent of the visible working name, SQLite access is owned by
a dedicated per-Project worker thread (WAL, `synchronous=FULL`, foreign keys
on), renames only report "Saved" after an acknowledged SQLite commit, a second
process cannot open the same Project while a lock is held, and a manual
backup can be created and restored as an independently editable copy with a
new Project ID. See `app/src-tauri/src/` for the module boundaries
(`domain`, `application`, `persistence`, `package`, `backup_recovery`,
`tauri_boundary`) and their automated tests.

Task 02A adds crash-recoverable schema-v1 migration with an external validated
pre-migration SQLite recovery point, coordinated database/manifest version
publication, and recovery after an interrupted manifest publication. Projects
now have stable UUIDv7 Categories, optional Category-local Types, and
record-registered Entries. Every Project has exactly one system-managed
Uncategorized Category; unnamed Entries remain valid and display an unstored
`[Unnamed Entry]` fallback. The plain Project Home supports listing and creating
Categories, Types, and Entries, inline Category/Type creation during Entry
creation, and revision-checked continuous saving of Entry names.

**Current non-goals** (deliberately out of scope for this slice): Fields,
Capabilities, Relationships, Spatial, Chapters/Story Units, TipTap prose
editing and rich descriptions, search/FTS, Tags/Roles/Statuses, aliases,
Archive/Trash, deletion/retirement, final Recent/Pinned navigation, and any
final visual design system.

### Manual native-close verification

In a packaged Tauri build, verify title-bar and OS close shortcuts: clean Projects
close their backend session before the window exits; dirty, saving, and failed
renames keep the window open until an explicit discard; backend-close failures
remain visible and do not exit; and the in-app **Close Project** button returns
to Home without exiting the application.

### Engineering spike notes affecting later slices

* **Project locking is age-based, not PID-liveness-based.** Cross-platform
  liveness checks for another process's PID are unreliable (especially over
  network/cloud-synced filesystems), so a lock is only considered "stale"
  after a fixed inactivity threshold; recovering a stale lock is always an
  explicit, separate operation and is never automatic.
* **`ProjectDbWorker` is a single dedicated OS thread per open Project**
  owning the one `rusqlite::Connection`, driven by a serialized job queue
  over `std::sync::mpsc` channels (no async pseudocode wrapping a shared
  connection). Spawning synchronously validates identity, pragmas, and
  migrations before the open call returns, so open failures are reported
  rather than surfacing later.
* **Backups are consistent snapshot directories, not compressed archives.**
  This slice uses SQLite's Online Backup API against the live worker
  connection (safe to run while WAL is active) and copies the manifest and
  managed package directories alongside it, but does not compress the result
  into a `.wcbackup` archive; a compression step can be layered on later
  without changing the snapshot/validation contract.
* **No file-picker dialogs are used yet.** The minimal UI takes filesystem
  paths as plain text input to avoid adding a dialog plugin dependency before
  it is genuinely needed.
