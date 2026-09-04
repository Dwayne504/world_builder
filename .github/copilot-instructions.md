# GitHub Copilot Instructions — The Worldcrafter

The Worldcrafter is a local-first desktop application for worldbuilding and story writing.

Before making architectural or product-relevant implementation decisions, read the applicable documentation under `/docs`.

The primary sources are:

* `docs/concept/The_Worldcrafter_Concept_V0.02.docx`
* `docs/milestones/MILESTONE_01_IMPLEMENTATION.md`

## Product behavior

Do not silently invent, simplify, or change product behavior when the documentation already defines it.

If a technical decision materially changes product behavior, expose the issue for review instead of choosing a new product rule without discussion.

Examples of product decisions that must not be silently changed:

* whether deleting a Type deletes authored values
* whether Categories determine Entry behavior
* whether relationships are stored once or duplicated
* whether renaming an Entry rewrites manuscript prose
* whether Spatial movement rewrites descendant records
* whether incomplete Entries are considered invalid

Technical implementation details may be chosen freely when they preserve documented behavior.

## Core invariants

Preserve these rules throughout the codebase.

### Stable identity

Visible names are never identifiers.

Renaming or reorganizing records must not break references.

### One source of truth

Do not store the same semantic relationship independently in multiple places.

Relationship-backed Fields must project/edit the underlying relationship rather than duplicate it.

### Preserve authored data

Template, Category, Type, or definition changes must not silently delete authored values.

### Derived data is derived

Backlinks, breadcrumbs, recursive Spatial ancestry, indexes, cached search results, and similar computed information must not become independent authoritative truth.

### Structural validity

Structural operations must not leave partially applied invalid states.

Spatial primary containment must remain acyclic.

### Optionality

Do not introduce unnecessary mandatory fields, setup requirements, completeness scores, or blocking validation.

Worldcrafter should allow incomplete and stub material.

### Local-first

Core functionality must not depend on an account, internet connection, or cloud service.

### Reversible by default

Prefer safe, reversible operations such as rename, move, Archive, Trash, reparent, and template detachment.

Permanent destructive operations require explicit handling.

## Approved Milestone 01 product decisions

### Project-scoped identity

Record identity is interpreted within its Project boundary.

Restore as Copy creates a new Project ID while preserving all snapshot-internal record and definition IDs. Internal references must remain valid without recursively regenerating IDs.

### References and semantic relationships

Use Relationships for semantic world facts requiring inverse meaning, metadata, roles, lifecycle/history, or semantic cardinality.

Plain Entry Reference Fields are reserved for non-semantic navigational or configurational pointers. They may generate generic backlinks but not semantic inverse projections.

### Soft cardinality

Relationship cardinality is a soft expectation in Milestone 01.

Conflicting authored relationships must be preserved and visibly flagged. They must never be silently overwritten, deleted, selected, or automatically repaired. Replacement requires an explicit user operation.

### Story links and Roles

A Story Unit–Entry link exists independently from its Roles.

One Story link may have zero, one, or multiple Roles. Multiple Roles must not create duplicate Story links.

## Scope discipline

Implement only the currently approved milestone scope.

Do not implement deferred systems merely because the architecture could support them.

Examples currently outside Milestone 01 include:

* timelines and fictional calendar systems
* editable maps
* graph visualization
* storyboards
* collaboration
* cloud sync
* mobile support
* full multi-party relationship editing
* advanced continuity systems
* AI worldbuilding features

Architecture may leave room for these systems without implementing them.

## Development approach

Prefer:

* clear domain boundaries
* stable domain IDs
* explicit invariants
* testable domain logic
* small incremental changes
* automated tests for core model behavior
* migrations rather than destructive assumptions
* rebuildable derived indexes/caches

Do not optimize for minimum code at the expense of the documented domain model.

## Product ambiguity

When implementation exposes an unresolved product question:

1. identify the ambiguity,
2. explain the technical consequences,
3. document reasonable alternatives,
4. stop short of committing to difficult-to-reverse behavior until the product decision is reviewed.

## Architecture-first rule

Do not begin application implementation until the current Milestone 01 architecture proposal has received explicit approval after technical review.

The existence or review of a proposal does not imply approval.

During the current architecture correction cycle, the expected proposal is:

`docs/architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL_V2.md`

Until that proposal is approved, perform documentation and architecture work only. Do not scaffold or implement the application.
