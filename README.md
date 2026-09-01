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
