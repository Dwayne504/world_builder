# The Worldcrafter — Milestone 01 Implementation Specification

**Document:** `MILESTONE_01_IMPLEMENTATION.md`  
**Milestone:** 01 — The First Real Worldcrafter  
**Status:** Ready for architecture proposal  
**Primary product source of truth:** `The_Worldcrafter_Concept_V0.02.docx`

---

## 0. Purpose of this document

This document translates **The Worldcrafter Concept V0.02** into an implementation-facing contract for the first real software milestone.

It is **not** a replacement for the concept document. The concept document defines the product philosophy, conceptual model, user-facing meaning, and the reasoning behind the decisions. This file defines what Milestone 01 must accomplish in implementation terms.

When this document and the concept document appear to conflict:

1. Prefer the **explicit product behavior** defined in Concept V0.02.
2. Do **not** silently invent a new interpretation.
3. Surface the ambiguity as a product question before implementing behavior that would be difficult to reverse.

The implementation process for this milestone must begin with an **architecture proposal**, not immediate feature coding.

---

# 1. Milestone objective

Milestone 01 must prove the central Worldcrafter loop:

> **Create material → structure it only where useful → connect it → navigate it from multiple directions → write with that material nearby → close and reopen the application without losing or corrupting anything.**

The first usable version does not need to realize the whole Worldcrafter vision. It must establish the durable foundations that later systems such as timelines, maps, storyboards, continuity checks, richer relationship knots, and advanced manuscript linking can build upon.

The milestone succeeds when a small but real fictional world can be represented naturally without fighting the model.

The canonical integration scenario uses:

- **Thron** — Character Entry
- **Singularity Blade** — Object Entry
- **Tortuga** — Creature Entry with Spatial capability
- **Northern Shell** — Spatial Entry
- **City of Arak** — Spatial Entry
- **Temple of the First Step** — Spatial Entry
- **The First Step** — Chapter Story Unit

---

# 2. Architectural process requirement

## 2.1 First Copilot/Codex task

Before implementing application code, inspect:

- this file,
- `The_Worldcrafter_Concept_V0.02.docx`,
- the existing repository,
- the existing technology choices, if any.

Then produce:

`docs/architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL.md`

The proposal must cover at least:

- application architecture,
- module boundaries,
- domain model,
- persistence strategy,
- stable-identity strategy,
- transaction / atomic-operation strategy,
- autosave strategy,
- backup/recovery approach,
- search/indexing approach,
- navigation/session architecture,
- proposed UI state architecture,
- migration/versioning strategy,
- testing strategy,
- implementation phases,
- material risks,
- unresolved product questions.

**Do not implement Milestone 01 before that architecture proposal has been reviewed.**

## 2.2 Product decisions versus technical decisions

Copilot/Codex may choose technical solutions for:

- storage technology,
- indexing implementation,
- transaction mechanism,
- UI component architecture,
- dependency injection,
- module layout,
- cache strategy,
- serialization details,
- test frameworks,
- internal naming conventions.

Copilot/Codex must **not silently decide product behavior** for questions such as:

- whether deleting a Type deletes field values,
- whether a Category defines behavior,
- whether `Owner` is stored separately from an Ownership relationship,
- whether manuscript prose changes after an Entry rename,
- whether moving a Spatial Entry rewrites all descendants,
- whether incomplete Entries are considered invalid.

If implementation reveals a product-level ambiguity, document it and request a product decision.

---

# 3. Product constitution — hard invariants

These invariants are more important than implementation convenience.

## 3.1 Stable identity

Visible names are **never** identifiers.

Project, Entry, Type, Field Definition, Relationship Definition, Story Unit, and other durable records that need identity must use stable internal IDs.

Renaming must never break references.

Moving an Entry between Categories or Types must never change its identity.

Reordering a Chapter must never change its identity.

## 3.2 Optionality

The system must not treat supported structure as mandatory merely because it exists.

Examples:

- Type is optional.
- most fields are optional.
- worldbuilding is optional.
- manuscript writing is optional.
- Spatial is optional.
- story planning is optional.
- empty fields are normal.
- incomplete / stub Entries are normal.

Do not add completion percentages, blocking validation, mandatory lore questionnaires, or “finish setup” gates.

## 3.3 One source of truth for relationships

A semantic relationship is stored once.

Example:

`Thron owns Singularity Blade`

must not be represented as two independent mutable facts:

- `Thron.owns = Blade`
- `Blade.owner = Thron`

Instead, one relationship instance must generate the forward and inverse views.

Relationship-backed fields are **projections/editors of the relationship**, not duplicate data.

## 3.4 Authored data survives template evolution

Category and Type templates may make fields, relationships, or capabilities available.

They do **not** own authored values.

Removing or changing a template must never silently delete authored data.

## 3.5 Derived context is not independent truth

Examples of derived information:

- breadcrumbs,
- backlinks,
- “Thron is somewhere within Tortuga,”
- indirect Story usage,
- search indexes,
- Saved View results,
- recursive descendant lists.

Authoritative authored structure must remain the source of truth. Derived data must be rebuildable where practical.

## 3.6 Structural operations remain valid atomically

Operations such as:

- spatial reparenting,
- relationship creation,
- Category moves,
- Type changes,
- Story Unit reorder,
- bulk field migration,
- permanent destructive operations,

must not leave partially applied invalid states after interruption.

An operation either completes or the previous valid state survives.

## 3.7 Reversible by default

Normal editing should favor reversible operations:

- rename,
- move,
- reparent,
- archive,
- Trash,
- Type change,
- Category change,
- remove field from template.

Permanent destruction must be exceptional and explicit.

## 3.8 Manuscript text remains authored prose

Renaming an Entry must not automatically rewrite ordinary manuscript text.

A linked mention may point to an Entry whose display name later changes while preserving the exact authored text shown in the manuscript.

---

# 4. Core conceptual model required by Milestone 01

## 4.1 Identifiable records

Do not force all durable things into a single user-facing “Entry” concept.

The architecture should support the broader idea of identifiable records so that different record families can share stable identity and linking infrastructure without pretending they are all the same kind of thing.

At minimum Milestone 01 includes:

- Project
- Entry
- Story Unit / Chapter
- Relationship Definition
- Relationship Instance
- Category
- Type
- Field Definition
- Tag / supporting definitions as needed

Later systems may add:

- rich multi-party relationship instances,
- temporal occurrences,
- Storyboards,
- Event-like records,
- map records.

The implementation should not make later record families impossible.

---

# 5. Project

## 5.1 Definition

A Project is the stable, self-contained workspace for one connected body of creative work.

A Project is **not inherently**:

- one book,
- one world,
- one series.

It may contain:

- worldbuilding only,
- story material only,
- both,
- zero or multiple future books,
- custom configuration,
- assets.

## 5.2 Required behavior

Milestone 01 must support:

- create Project,
- working name,
- optional starter preset,
- blank Project,
- stable Project ID,
- rename Project,
- open existing Project,
- recent Projects,
- Project Home,
- Project-local definitions,
- local-first operation,
- self-contained persistence.

Project names do not need to be globally unique.

## 5.3 Project creation

Minimal flow:

- Working name
- optional starting preset

Suggested presets:

- Blank
- Worldbuilding
- Writing
- Worldbuilding + Writing

Presets are starter configurations only. They must not permanently lock the Project into a mode.

Do not require:

- book title,
- world title,
- series title,
- account,
- network connection.

After creation, open Project Home rather than Project Settings.

---

# 6. Entry

## 6.1 Definition

An Entry is an independently identifiable piece of world material that can have its own content and can be referenced from elsewhere.

Examples:

- Thron
- Singularity Blade
- Tortuga
- a religion
- a faction
- a species
- a city
- a major historical event, later

Not every structured fact is an Entry.

Examples that are not Entries:

- `Eye color = green`
- `Age = 43`
- the connection `Thron owns Blade`
- arbitrary prose inside Thron's description

## 6.2 Required Entry properties

Internally required:

- stable ID
- Project
- Category

Presentation requires:

- display name or temporary display label

Optional:

- Type
- description
- field values
- relationships
- tags
- capabilities
- assets
- story links
- statuses

## 6.3 Temporary / unnamed Entries

The application must allow quick creation of incomplete Entries.

Examples:

- `[Unnamed Character]`
- `Thron's mother`
- `Unknown Imperial Officer`

The internal stable ID provides identity before the final name exists.

## 6.4 Entry editor priorities

The Entry editor should visually prioritize:

1. name / identity,
2. free-form description,
3. structured fields,
4. relationships,
5. tags / supporting metadata,
6. contextual information.

It must not feel like a mandatory questionnaire.

---

# 7. Category

## 7.1 Definition

A Category is the author's primary organizational home for an Entry.

It answers:

> “Where does the author primarily file this Entry?”

It does **not** answer:

> “What specialized behavior can this Entry perform?”

Examples:

- Characters
- Objects
- Places
- Creatures
- Organizations
- Evidence
- Technology
- Religions

## 7.2 Rules

- Exactly one primary Category per Entry.
- Category can be changed without changing Entry identity.
- Categories are Project-local.
- Categories may provide default Fields.
- Categories may provide default Capabilities.
- Categories may organize available Types.
- Categories do not make fields mandatory.
- Categories do not determine all Entry behavior.
- Categories are flat in the normal Milestone 01 UI.
- Do not implement nested Category hierarchies as a core requirement.

## 7.3 Uncategorized

Every Entry must have a Category internally, but the user does not always need to choose one.

Provide a system-managed `Uncategorized` fallback.

It may be hidden when empty.

## 7.4 Category deletion

A Category containing Entries cannot simply disappear.

Before removal, Entries must be reassigned, e.g.:

- another Category,
- `Uncategorized`.

Deleting a Category must not delete its Entries.

---

# 8. Type

## 8.1 Definition

A Type is an optional, more specific classification/template inside one Category.

Examples:

- Character → Human
- Object → Weapon
- Weapon → Sword
- Creature → World Turtle
- Place → City

A Type is not intended to represent every possible descriptive dimension.

Examples usually better represented elsewhere:

- Main Character → Story Role
- POV → Story Role
- Favorite → Tag
- Canon → Status
- Current Owner → Relationship

## 8.2 Rules

- Type is optional.
- One primary Type per Entry.
- Type belongs to one Category.
- User-defined Types are supported.
- Optional parent Type may exist.
- Type may provide default Fields.
- Type may provide suggested Relationships.
- Type may provide default Capabilities.
- Type changes never silently delete authored information.
- Type name is not identity.

Do not build the user experience around deep inheritance trees, even if the internal model can safely support parent chains.

## 8.3 Template behavior

A Type template defines what structure is useful / available.

It does not define what data is legally allowed to exist on an Entry.

Changing Type:

- new template structure becomes available,
- old empty inherited fields may disappear,
- populated authored values survive,
- incompatible Type/Category combinations require explicit handling.

---

# 9. Fields

## 9.1 Definition

A Field exposes one structured piece of information about an Entry.

Separate:

- **Field Definition** — what the field means and how it behaves.
- **Field Value** — the actual value for one Entry.

## 9.2 Required Milestone 01 field kinds

Implement:

- Short Text
- Rich Text
- Number
- Choice
- Multi-choice
- Boolean
- Entry Reference

Temporal values are deferred unless a lightweight placeholder is architecturally useful.

Images / attachments are not core Field types. Assets are conceptually separate.

## 9.3 Field Definition

A Field Definition should support as relevant:

- stable ID,
- name,
- help text / description,
- value kind,
- cardinality,
- unit metadata,
- allowed choice values,
- reference constraints,
- presentation section,
- ordering,
- visibility defaults,
- source / template provenance,
- relationship projection metadata when relationship-backed.

## 9.4 Field origin

Fields may come from:

1. Category defaults
2. Type defaults
3. Entry-local custom fields

The Entry editor presents a merged useful view. Users should not need to care about provenance during normal editing.

## 9.5 Empty inherited fields

A template may make a field available without creating a stored empty Field Value for every Entry.

Example:

Human provides `Eye color`.

Thron has no Eye color value yet.

The UI can show an empty available field while the persistence model stores no authored value until one exists.

## 9.6 Local field promotion

A local Field may be promoted to:

- the Entry's Type,
- the Entry's Category.

Example:

Tortuga gets local `Shell diameter`.

Later:

`Make available to Type: World Turtle`

Tortuga keeps its value. Other World Turtle Entries gain the field as available.

## 9.7 Template evolution rules

### Additive changes

Example:

Add `Eye color`.

Safe to make available on applicable Entries.

### Cosmetic changes

Example:

Rename `Birth Planet` → `Birthplace`.

Safe if semantics are unchanged.

Stable Field Definition identity remains the same.

### Restrictive changes

Examples:

- Multiple → Single
- Any Character → Human only

New constraints apply to future editing.

Existing conflicting values survive and are surfaced for review.

### Semantic migrations

Example:

Short Text → Number.

Requires explicit migration / conversion review.

Do not silently coerce invalid values.

### Destructive changes

Deleting actual stored authored values must be explicit.

## 9.8 Retired field definitions

Removing a field from a Type or Category should not destroy existing values.

Prefer retiring/detaching the shared definition from future template use while preserving historical Entry usage.

Do not generate hundreds of unrelated clones unless technically necessary.

A retired definition may later be restored.

## 9.9 Choice values

Choice options should ideally have stable identity independent from their display labels.

This allows:

- safe rename,
- retirement,
- restoration.

Removing an option that existing Entries use should preserve those historical values as retired or require explicit replacement.

## 9.10 Shared definition editing

Editing one Entry value must be visually and behaviorally distinct from modifying the shared Field Definition.

The UI must not make “edit Thron's Age” and “change the Human Age field for every Human” feel like the same action.

---

# 10. Relationships

## 10.1 Definition

A Relationship is a typed semantic connection among identifiable records.

Milestone 01 primarily implements binary Entry-to-Entry relationships, but the architecture must leave room for richer relationship instances later.

Examples:

- Owns ↔ Owned by
- Child of ↔ Parent of
- Member of ↔ Has member
- Adjacent to ↔ Adjacent to
- Opposes ↔ Opposed by
- Located in ↔ Contains / contextual inverse

## 10.2 Relationship Definition

Support as relevant:

- stable ID,
- semantic name,
- forward label,
- inverse label,
- directed or symmetric behavior,
- allowed source/target constraints,
- cardinality expectations,
- presentation grouping,
- optional field-projection metadata.

## 10.3 Relationship Instance

Milestone 01 relationship instances must have stable identity sufficient for safe editing and metadata attachment.

Support:

- Relationship Definition
- participant A
- participant B
- direction / roles derived from definition
- optional note/basic metadata

Later instances may support:

- participant roles,
- dates,
- status,
- custom attributes,
- multiple participants,
- event/context links.

Do not choose a storage representation that makes later richer relationship instances prohibitively difficult.

## 10.4 Directed and symmetric

Support both:

Directed:

`Thron owns Blade`

Inverse:

`Blade is owned by Thron`

Symmetric:

`Arak adjacent to Glass Forest`

The user should not need to think about forward/inverse implementation direction during ordinary editing.

## 10.5 Soft constraints

Cardinality and target constraints normally guide rather than police.

Example:

Ownership may normally expect one current owner.

If the user attempts two, warn instead of automatically deleting one.

Hard validation should be reserved for structurally impossible states.

## 10.6 Relationship-backed fields

Some relationships should be presented like properties.

Examples:

- Mother
- Birthplace
- Current Owner
- Homeworld

A relationship-backed Field is a view/editor over Relationship Instances.

It must **not** store a second independent target reference.

Changing the field changes the relationship.

Changing the relationship changes the field projection.

## 10.7 Presentation de-duplication

Do not show the same relationship twice by default.

Example:

If `Current Owner: Thron` is presented as a field on the Blade, do not also show `Owned by → Thron` in the main Relationship section unless the user explicitly requests a full/raw relationship view.

## 10.8 Quick-create target Entries

When a referenced target does not exist:

Example:

`Birthplace: Kharon`

Offer:

`Create Place "Kharon"`

Create a stub Entry, link it, and return immediately to the original editor.

Do not force the user to finish Kharon first.

## 10.9 Organic relationship definition creation

A user may create a new Relationship Definition during use.

Example:

`Despises`

Quick creation should be lightweight.

Advanced constraints may be configured later.

## 10.10 Simple → rich evolution

Adding a note or metadata to a simple relationship must not require replacing its identity.

The relationship can become richer over time.

---

# 11. Future relationship “knots” — architectural compatibility only

Milestone 01 does **not** require a full multi-party relationship UI.

However the architecture must not assume that every meaningful connection will forever be a trivial immutable A→B edge.

Future example:

Temporary Alliance:

- Thron — ally
- Leopold — ally
- Wanderer — opposing force
- Context — Battle of Xeran
- Start / End
- Notes

This may become a rich multi-party Relationship Instance.

Milestone 01 should preserve this future path.

---

# 12. Capability system

## 12.1 Definition

A Capability is a system-defined, composable package of specialized Entry behavior.

Capability answers:

> “What specialized tools/structures can this Entry use?”

It does not answer:

> “What is this thing?”

## 12.2 Milestone 01 capabilities

Implement:

- Base
- Spatial

Do not implement Event capability yet.

## 12.3 Rules

- Every Entry has Base behavior.
- Specialized Capabilities are system-defined.
- Categories may provide default Capabilities.
- Types may provide default Capabilities.
- Individual Entries may add/remove compatible Capabilities.
- Category and Type do not change when a Capability is added.
- Capabilities compose the editor rather than replacing Entry identity.

Example:

Tortuga:

- Category: Creatures
- Type: World Turtle
- Capabilities: Base + Spatial

The user-facing term may later be “Features,” while the internal/product concept remains Capability.

---

# 13. Spatial capability

## 13.1 Definition

Spatial allows an Entry to act as a navigable fictional-space location/container without forcing that Entry into the Places Category.

Canonical example:

Tortuga is a Creature and also contains cities.

## 13.2 Primary spatial hierarchy

A Spatial Entry may have:

- zero or one primary spatial parent,
- zero or more direct Spatial children.

Root Spatial Entries are valid.

Primary spatial hierarchy must remain acyclic.

## 13.3 Required operations

Milestone 01 must support:

- add Spatial capability,
- assign primary spatial parent,
- create child location,
- add existing location as child,
- show direct children,
- show ancestry breadcrumbs,
- show recursive descendants,
- reparent,
- preserve descendant identity during reparenting,
- prevent containment cycles.

## 13.4 Structural placement versus current location

Keep these distinct.

Structural:

`Temple is inside Arak`

Current / situational:

`Thron is currently in Temple`

A moving Spatial Entry such as Tortuga or a starship may have a current location without changing its internal child hierarchy.

## 13.5 Non-Spatial Entries may have location relationships

Thron does not become Spatial because he can stand inside the Temple.

The Blade does not become Spatial because it is stored in a vault.

Spatial targets may be referenced through semantic relationships such as:

- Current Location
- Stored At
- Based In
- Born In
- Operates In

These relationships remain semantically distinct.

## 13.6 Derived ancestry

Store:

`Thron → Current Location → Temple`

Do not redundantly store:

- Thron in Arak
- Thron in Northern Shell
- Thron in Tortuga

Derive those through Spatial ancestry.

If Arak is reparented, Thron's broader context updates automatically.

## 13.7 Moving Spatial parents

If Tortuga's current location changes:

`Emerald Sea → Western Wastes`

Do not rewrite every descendant.

Derived broader context changes through Tortuga.

## 13.8 Spatial connections

Non-hierarchical Spatial connections use the general Relationship system.

Examples:

- Adjacent to
- Connected to
- Leads to

They must not alter primary containment.

## 13.9 Spatial removal

Milestone 01 may restrict removing Spatial if capability-specific spatial data still depends on the Entry.

Do not silently discard spatial children or links.

---

# 14. Search

## 14.1 Project-wide search

Search across as appropriate:

- Entry display names,
- aliases,
- descriptions,
- Field values,
- Story Unit titles,
- manuscript text,
- structured references.

## 14.2 Ranking

Exact identity/name matches should rank above ordinary textual occurrences.

Alias matches should strongly resolve to the owning Entry.

Do not bury `Thron` the Entry under hundreds of manuscript text matches.

## 14.3 Result grouping

Distinguish at least:

- Entries
- Story Units
- structured matches
- manuscript / plain-text matches

Structured references must not be presented as equivalent to guessed text matches.

---

# 15. Backlinks and contextual navigation

## 15.1 Backlinks

Backlinks are derived automatically.

Opening a record should reveal meaningful incoming references grouped by source or semantics.

Examples on Thron:

- owns / is owner of Blade
- Current Location
- Story usage
- related Entries

Examples on Tortuga:

- contained locations
- Characters somewhere within
- Story usage within descendant locations

## 15.2 Direct versus indirect

Default context emphasizes direct connections.

Indirect graph traversal should not flood normal Entry views.

Spatial recursive context is an explicit useful exception because hierarchy gives the path clear meaning.

## 15.3 Context preservation

Following a connection must be fast and reversible.

Milestone 01 should support:

- Back
- Forward
- basic tabs
- Recent
- Pin
- preserving prior search/explore state when navigating away

The exact visual mechanism may vary, but clicking the Blade from Thron must not make it painful to return to Thron.

---

# 16. Explore

Milestone 01 should include a structured Explore surface.

Filters should include at least:

- Category
- Type
- Capability
- Tags, if implemented in the initial UI
- Relationship
- Location
- exact location vs anywhere within Spatial hierarchy

Search and Explore are conceptually distinct:

- Search = find something approximately known.
- Explore = browse/filter structured material and connections.

## 16.1 Initial result presentations

Required:

- List

Useful if technically reasonable:

- Table

Deferred:

- Cards as primary requirement
- Graph view

## 16.2 Missing / incomplete material

Explore may support useful filters such as:

- stubs
- field is empty
- archived
- unresolved

Do not translate this into completion scores.

## 16.3 Saved Views

Saved Views are an early-follow-up / stretch feature for Milestone 01.

If implemented, they store live queries, not copied result lists.

They must reference stable IDs in filters rather than display names.

---

# 17. Aliases

Entries should support simple aliases.

Example:

Thron:

- The Ghost
- Godhunter

Aliases should:

- resolve in search,
- help disambiguation,
- remain alternative names for the same Entry identity.

Advanced contextual/language/time-bound alias behavior is deferred.

---

# 18. Tags, Roles, and Statuses

These concepts must remain distinct even if Milestone 01 implements only a subset of their full UI.

## 18.1 Tags

Loose, user-defined labels for filtering/organization.

They carry no structural behavior.

Examples:

- needs research
- favorite
- godhunter

## 18.2 Roles

Contextual functions.

Examples:

- Thron is POV in a Chapter
- Thron is Main Character in a Book
- participant role in a future rich Relationship Instance

A Role is not an intrinsic Type.

## 18.3 Statuses

Controlled values inside a named Status System.

Examples:

- Writing: Drafting
- Canon: Canon
- Research: Needs Review

Do not implement one universal `Status` field that mixes unrelated systems.

Full claim-level canon/uncertainty handling is deferred.

---

# 19. Story Units — Milestone 01 scope

## 19.1 Definition

A Story Unit is an identifiable ordered piece of narrative structure.

It is not an ordinary world Entry.

Milestone 01 focuses on **Chapters**.

## 19.2 Required Chapter behavior

Support:

- create loose Chapter,
- stable Chapter ID,
- editable title,
- explicit ordering,
- manuscript prose,
- Plan / Notes area,
- word count,
- autosave,
- archive / Trash,
- Chapter-level links to Entries,
- contextual roles such as POV / Setting,
- Story Usage backlinks on Entries,
- reorder Chapter list.

A Chapter does not require a Book.

## 19.3 Deferred Story hierarchy

Do not require implementation yet of:

- Book
- Part
- Scene
- Act
- Episode
- arbitrary hierarchy engine
- Storyboard
- alternate manuscript branches

The architecture should leave a future path for richer Story Units.

## 19.4 Reading order versus chronology

Milestone 01 does not implement timelines, but the design must preserve the rule:

Narrative reading order is independent from fictional chronological order.

Do not model Chapter order as timeline order.

---

# 20. Chapter links to world material

A Chapter may explicitly link to:

- Characters
- Places
- Objects
- other Entries

The link may include a Story Role such as:

- Appears
- POV
- Setting
- Primary Setting
- Mentioned, later if useful

Story links must be bidirectional.

Example:

Chapter:

`Thron — POV`

Thron:

`Story Usage → The First Step`

One connection, not duplicated mutable data.

---

# 21. Spatial story context

If Chapter explicitly links:

`Temple of the First Step — Setting`

the UI may derive:

`Tortuga > Northern Shell > Arak > Temple`

Do not require explicit duplicate links to every ancestor.

Keep explicit and derived Story usage distinguishable.

Tortuga may show:

`The First Step — used within Tortuga via Temple`

without pretending Tortuga was directly selected as the Chapter setting.

---

# 22. Manuscript context panel

The writing surface is primary.

Milestone 01 must provide a way to see linked world material nearby without leaving the manuscript.

Minimum acceptable behavior:

- collapsible right context panel,
- linked Entry list,
- quick preview of relevant Entry information,
- spatial path for setting,
- open full Entry from context.

The manuscript cursor/scroll state must be preserved when opening/closing context.

## 22.1 Split view

Split view is a Milestone 01 stretch goal / very early follow-up.

If technically straightforward, include it.

Do not delay the first usable Chapter editor for a complex multi-pane layout system.

## 22.2 Focus mode

A simple way to collapse side panels and focus on prose is desirable.

---

# 23. Passage-level links — deferred from hard Milestone 01 scope

The architecture should allow future exact text-span links:

`Thron` in prose → Entry ID

Important future behavior:

- authored visible text survives Entry rename,
- links remain structural,
- selected manuscript text may quick-create a new Entry.

Do not make robust passage-level linking a blocker for Milestone 01 unless implementation is clearly cheap and safe.

---

# 24. Project / Entry / Chapter home surfaces

Milestone 01 should provide the following major surfaces.

## 24.1 Application Home

Purpose:

- recent Projects,
- New Project,
- Open Project.

## 24.2 Project Home

Purpose:

- continue work,
- Recent,
- Pinned,
- create Entry,
- create Chapter,
- create Category.

Avoid analytics-heavy dashboard design.

## 24.3 Entry Editor

Shared layout concept:

- left Project navigation,
- center Entry editor,
- right optional context.

Entry capabilities add tools/sections rather than replacing the Entry.

## 24.4 Spatial surface

Within Spatial-enabled Entry:

- parent/current location,
- hierarchy,
- children,
- breadcrumbs,
- add/move location,
- contextual records.

## 24.5 Chapter Editor

- manuscript primary,
- Plan/Notes,
- linked world context,
- collapsible side panels.

## 24.6 Explore/Search

- search,
- structured filters,
- results.

## 24.7 Structure Configuration

Manage:

- Categories,
- Types,
- Fields,
- Relationship Definitions.

Normal creative workflows must not require visiting this configuration surface.

---

# 25. Organic creation — mandatory interaction philosophy

Worldcrafter must allow the model to grow while the author works.

Examples:

## 25.1 Create missing Category inline

While creating Tortuga:

`Category → Create "Creatures"`

Return immediately to Tortuga.

## 25.2 Create missing Type inline

`Type → Create "World Turtle"`

Return immediately.

## 25.3 Create missing referenced Entry inline

Thron:

`Birthplace → Kharon`

Kharon does not exist.

Create Place `Kharon`.

Return immediately to Thron.

## 25.4 Promote local structure later

Tortuga:

`Shell diameter = 900 km`

Later:

`Make available to Type: World Turtle`

The model should support this without re-entering data.

The application must preserve momentum and avoid unnecessary detours into configuration screens.

---

# 26. Navigation requirements

Milestone 01 should support:

- left Project navigation,
- open record,
- Back,
- Forward,
- basic tabs,
- Recent,
- Pin,
- search/quick navigation reuse where practical.

Tabs do not need advanced grouping.

Opening a reference must not destroy the user's current thought context.

---

# 27. Archive, Trash, retire, and deletion

Keep these meanings distinct.

## 27.1 Archive

Applies to authored records such as:

- Entries
- Chapters

Meaning:

> Hide from normal active workspace without deleting or invalidating references.

Archived targets remain resolvable and visibly marked.

## 27.2 Trash

Recoverable deletion state for authored records.

Trashed targets may remain resolvable until permanent deletion.

Restore should recover the record with links intact.

## 27.3 Retire

Applies to reusable definitions/options such as:

- Field Definitions
- Relationship Definitions
- Choice options
- potentially Types

Meaning:

> Stop offering for new use while preserving existing historical uses.

## 27.4 Permanent deletion

Exceptional action.

Before permanent deletion:

- show important dependencies,
- preserve structural validity,
- provide recovery safeguards where possible.

Semantic references to a permanently deleted target may become unresolved rather than disappearing silently.

---

# 28. Unresolved references

A permanently missing semantic target may leave an unresolved reference.

Preserve minimal useful metadata such as:

- last-known display name,
- former record kind / Category if helpful,
- relationship semantics.

Offer repair actions later such as:

- reconnect,
- create replacement,
- remove reference.

Do not automatically reconnect solely because a new Entry has the same name.

Names are not identity.

Structural hierarchy should be repaired explicitly rather than casually leaving broken primary parent structure.

---

# 29. Reparenting and destructive spatial operations

## 29.1 Reparenting

Moving Arak from Tortuga to another parent:

- keeps Arak ID,
- keeps descendants,
- keeps relationships,
- keeps Story links,
- updates breadcrumbs,
- updates derived broader location context.

## 29.2 Deleting a Spatial parent

Moving a Spatial Entry to Trash does not need to automatically Trash all descendants.

Permanent deletion must require explicit handling of direct children:

- move to another parent,
- make root,
- delete subtree intentionally.

Do not silently cascade permanent deletion through a hierarchy.

---

# 30. Autosave and persistence

## 30.1 Continuous autosave

Core content saves continuously:

- manuscript,
- descriptions,
- field values,
- relationships,
- hierarchy changes,
- Story links,
- metadata.

No normal “Save changes?” workflow.

## 30.2 Saved indicator contract

`Saved` means the change has reached durable Project storage.

Do not display Saved merely because UI state changed.

## 30.3 Manuscript priority

Manuscript text receives especially aggressive persistence.

Closing immediately after typing should preserve the recent text.

## 30.4 Atomic operations

Structural operations must not leave partial states after crash/interruption.

---

# 31. Crash recovery and session recovery

On unclean shutdown:

- recover automatically when possible,
- preserve valid Project state,
- avoid unnecessary recovery wizard UX.

A small notice is sufficient if recovery was routine.

Useful session restoration:

- last active Project,
- last active record,
- open tabs if practical,
- manuscript cursor/scroll state if practical.

---

# 32. Safety layers

Keep separate:

## Undo

Immediate local editing mistakes.

## Version history

Older content / structural states.

Full UI deferred, but architecture should not make it impossible.

## Trash

Recover deleted records.

## Backup

Recover damaged/lost Project.

None of these substitutes for the others.

---

# 33. Backups

Milestone 01 should include practical Project backup.

Required:

- rolling automatic local backups,
- manual “Create Backup Now,”
- whole-Project snapshots,
- restore backup,
- safe default restore-as-copy where feasible.

High-impact migrations or destructive operations should be capable of creating a safety snapshot first.

Backup includes:

- Project definitions,
- Entries,
- Story Units,
- relationships,
- spatial structure,
- manuscript,
- relevant assets,
- settings.

A manuscript export is not a backup.

---

# 34. Project portability

Project data should be self-contained and portable by default.

Moving/copying the Project should not break internal links.

Avoid absolute-path coupling for internal references.

## 34.1 Assets

Default future attachment behavior should copy/import assets into managed Project storage.

External linked files may be supported later explicitly.

The first storage design should not assume every attachment will forever live at an external absolute path.

---

# 35. Project format versioning

Persist Project format/version metadata.

Future data-model migrations must:

- be deliberate,
- create recovery point where appropriate,
- avoid partial migration,
- prevent older incompatible applications from blindly writing newer-format data.

Exact migration system is an architecture decision.

---

# 36. Search indexes and caches

Search indexes, backlink caches, recursive spatial caches, and similar derived structures must not become the only source of authored truth.

They must be rebuildable where practical.

---

# 37. Save failure behavior

If durable persistence fails due to disk, permission, or device problems:

Worldcrafter must stop claiming changes are saved.

Display a prominent warning:

> Changes are not being saved.

Preserve in-memory / emergency recovery data where technically feasible.

This is a high-priority trust requirement.

---

# 38. Local-first requirement

Core functionality must work without:

- account,
- cloud,
- internet connection.

This includes:

- open Project,
- edit world material,
- write manuscript,
- search,
- save,
- backup locally.

Future sync/collaboration must sit on top of the local-first core rather than becoming a dependency.

---

# 39. Explicit Milestone 01 non-goals

Do **not** expand Milestone 01 into these systems unless separately approved.

## Deferred world/time systems

- advanced fictional timelines
- fictional calendars
- Event Capability implementation
- event state-change system
- historical relationship evaluation
- age-at-time calculations
- continuity checking

## Deferred spatial systems

- editable maps
- coordinates
- polygon regions
- route simulation
- travel-time engine
- multiple primary spatial hierarchies
- full moving-location history

## Deferred relationship systems

- full multi-party knot editor
- relationship graph visualization
- advanced role schemas
- complex claim/source systems
- character-belief graph
- historical ownership UI

## Deferred story systems

- Books / Parts / Scenes as full hierarchy
- Storyboards
- lane/beat planner
- manuscript alternate branches
- exact passage-level Entry links as a hard requirement
- advanced publishing/export layout
- EPUB production

## Deferred platform/product systems

- collaboration
- cloud sync
- mobile companion
- shared live Entries across Projects
- custom user-programmable Capabilities/plugins
- AI-generated lore as a core workflow
- AI automation
- theme customization as a priority
- real-time multiplayer editing

These ideas may influence architecture. They do not automatically receive implementation tasks.

---

# 40. Milestone 01 canonical integration dataset

Use this dataset for integration tests and manual QA.

## 40.1 Project

**Thron**

## 40.2 Categories

### Characters
Contains:
- Thron

### Objects
Contains:
- Singularity Blade

### Creatures
Contains:
- Tortuga

### Places
Contains:
- Northern Shell
- City of Arak
- Temple of the First Step

## 40.3 Types

### Characters
- Human

### Objects
- Weapon

### Creatures
- World Turtle

### Places
- Region
- City
- Temple

## 40.4 Entries

### Thron

- Category: Characters
- Type: Human
- Description: non-empty
- Current Location: Temple of the First Step
- Owns: Singularity Blade
- optional Tag: godhunter

### Singularity Blade

- Category: Objects
- Type: Weapon
- Current Owner: Thron
- relationship is the inverse projection of the same Ownership instance

### Tortuga

- Category: Creatures
- Type: World Turtle
- Capabilities: Base + Spatial

### Northern Shell

- Spatial
- primary parent: Tortuga

### City of Arak

- Spatial
- primary parent: Northern Shell

### Temple of the First Step

- Spatial
- primary parent: City of Arak

## 40.5 Chapter

### The First Step

- loose Chapter
- manuscript text exists
- explicitly linked to Thron
- Thron role: POV
- explicitly linked to Temple of the First Step
- Temple role: Setting
- explicitly linked to Singularity Blade

Derived setting context should include:

`Tortuga > Northern Shell > City of Arak > Temple of the First Step`

---

# 41. Primary end-to-end acceptance test

The following workflow must succeed.

## Project creation

1. Create Project `Thron`.
2. Close and reopen.
3. Project remains available.

## Create Thron

4. Create Entry `Thron`.
5. Category: Characters.
6. Type: Human.
7. Write description.
8. Add a normal structured value.

## Create Blade through relationship flow

9. From Thron, add Relationship `Owns`.
10. Search for `Singularity Blade`.
11. It does not exist.
12. Quick-create Object `Singularity Blade`.
13. Return automatically to Thron.
14. Ownership exists.

Verify:

- Thron shows the Blade.
- Blade shows Thron as owner.
- only one semantic relationship is stored.

## Create Tortuga

15. Create Entry `Tortuga`.
16. Category: Creatures.
17. Type: World Turtle.
18. Add Spatial capability.

Verify:

- Tortuga remains in Creatures.
- adding Spatial did not force Category change.

## Build spatial hierarchy

19. Create Northern Shell inside Tortuga.
20. Create City of Arak inside Northern Shell.
21. Create Temple of the First Step inside Arak.

Verify Temple breadcrumb:

`Tortuga > Northern Shell > City of Arak > Temple of the First Step`

## Locate Thron

22. Set Thron Current Location = Temple.

Verify:

- direct location = Temple,
- derived ancestors = Arak, Northern Shell, Tortuga,
- no duplicate direct location facts are stored for the ancestors.

## Navigation

23. Open Thron.
24. Navigate to Blade.
25. Back returns to Thron.
26. Navigate to Temple.
27. Breadcrumb opens Arak / Northern Shell / Tortuga.
28. Search `Thron`.
29. Thron Entry ranks above ordinary manuscript text mentions.

## Chapter

30. Create loose Chapter `The First Step`.
31. Write prose.
32. Link Thron as POV.
33. Link Temple as Setting.
34. Link Singularity Blade.
35. Context panel shows linked world material.
36. Context displays Temple spatial ancestry.
37. Open Entry preview/full Entry without losing Chapter writing state.

## Persistence

38. Close application immediately after recent edits.
39. Reopen Project.

Verify:

- all Entries,
- relationships,
- hierarchy,
- Chapter,
- manuscript,
- links,
- navigation metadata needed by the product,

remain correct.

---

# 42. Tortuga structural acceptance test

Starting state:

`Tortuga > Northern Shell > Arak > Temple`

Thron directly located in Temple.

## Reparent

1. Create new Spatial Entry `Floating Continent`.
2. Move Arak under Floating Continent.

Verify:

- Arak ID unchanged,
- Temple ID unchanged,
- Thron still directly located in Temple,
- Temple breadcrumb changes,
- Thron broader context changes,
- no Thron record rewrite required for each ancestor.

## Move back

3. Move Arak back under Northern Shell.

Verify state restores coherently.

## Cycle prevention

4. Attempt to place Tortuga under Temple.

Must fail.

No “use anyway.”

Primary Spatial hierarchy must remain acyclic.

---

# 43. Rename torture test

Perform:

- Thron → Thron Godhunter
- Tortuga → A'Tor
- Creatures → Beings
- World Turtle → Worldback
- Shell diameter → Shell width
- Owns → Possesses

Verify:

- stable relationships survive,
- backlinks survive,
- breadcrumbs update,
- search finds renamed records,
- IDs remain unchanged,
- Chapter-level links survive,
- ordinary manuscript prose is not rewritten automatically,
- saved structural filters, if implemented, remain ID-based rather than name-based.

---

# 44. Template evolution acceptance test

1. Human contains `Age` as Short Text.
2. Thron value = `"43"`.
3. Another Character value = `"Unknown"`.
4. Migrate Age to Number.

Expected:

- valid value can become `43`,
- incompatible `"Unknown"` is not silently discarded,
- migration requires explicit handling/review.

Then:

5. Add `Eye color` to Human.
6. Existing Humans show it as available.
7. Thron sets `Eye color = Green`.
8. Remove Eye color from Human template.

Expected:

- Thron's Green value survives.
- Future Humans no longer automatically receive the field.
- historical use remains understandable.
- restoring the retired field can reconnect the shared definition.

---

# 45. Relationship acceptance test

Create one Ownership instance:

`Thron owns Singularity Blade`

Verify:

- Thron forward view resolves.
- Blade inverse view resolves.
- editing the owner updates the same relationship.
- deleting the relationship removes both projections.
- adding a note does not require replacing relationship identity.
- target/cardinality constraints do not silently destroy conflicting authored data.

---

# 46. Archive / Trash acceptance test

## Archive Blade

Verify:

- Blade hidden from normal active lists as designed.
- relationship from Thron remains valid.
- reference visibly indicates archived state.

## Unarchive

Verify complete restoration.

## Trash Blade

Verify:

- recoverable,
- relationships remain associated,
- restore returns everything intact.

## Permanent deletion

If implemented in Milestone 01:

- dependency impact is shown,
- semantic references become unresolved or are handled explicitly,
- no silent cascading deletion.

---

# 47. Crash / persistence acceptance test

Test at least:

## Manuscript

1. Type text.
2. Immediately close / kill process.
3. Reopen.

Expected:
recent manuscript survives or explicit recovery data is offered.

## Relationship

1. Create ownership relation.
2. Interrupt around operation boundary.

Expected:
either relationship exists completely or does not exist; never half-state.

## Spatial reparent

1. Move Arak.
2. Interrupt.

Expected:
either old hierarchy or new valid hierarchy; never two primary parents / invalid cycle / lost descendants.

---

# 48. Backup acceptance test

1. Create manual Project backup.
2. Make meaningful changes:
   - rename Entry,
   - delete/trash record,
   - change Chapter.
3. Restore old backup as copy.

Expected:

- recovered Project opens independently,
- old data is coherent,
- original newer Project remains intact.

---

# 49. Search / backlinks acceptance test

Given canonical dataset:

Search `Thron`:

- Thron Entry is a high-priority result.

Open Blade:

- backlink/context shows Thron.

Open Thron:

- Blade visible.
- Temple visible as direct Current Location.
- Arak / Northern Shell / Tortuga visible as derived location context.

Open Tortuga:

- Explore/Spatial context can discover Thron somewhere within descendants.

Search text mention without structured link:

- appears as text result,
- does not pretend to be confirmed structured Story usage.

---

# 50. Required tests and engineering quality

The architecture proposal must define a test strategy.

At minimum, automated tests should strongly cover domain invariants:

- stable identity,
- Relationship forward/inverse consistency,
- no duplicated relationship truth,
- Spatial cycle prevention,
- reparenting,
- derived ancestry,
- template-value preservation,
- Type/Category moves,
- persistence round trips,
- atomic operation boundaries where testable,
- archive / Trash / restore,
- Project reopen,
- search/index rebuild behavior where applicable.

Prefer domain-level tests that do not depend on UI rendering for core correctness.

UI tests should cover the most important end-to-end workflows.

---

# 51. Implementation-phase guidance

The final phases must be proposed by the architecture review.

A plausible direction is:

## Phase 0 — technical foundation

- app skeleton,
- Project loading,
- persistence foundation,
- stable identity primitives,
- transaction/unit-of-work strategy,
- test foundation.

## Phase 1 — Project / Category / Type / Entry

- create/open Project,
- Project Home,
- Categories,
- Types,
- Entry editor,
- description,
- basic navigation.

## Phase 2 — Fields

- definitions,
- values,
- template availability,
- local fields,
- promotion,
- safe evolution foundations.

## Phase 3 — Relationships

- definitions,
- instances,
- forward/inverse projection,
- relationship-backed fields,
- quick-create targets.

## Phase 4 — Spatial

- capability,
- hierarchy,
- breadcrumbs,
- reparenting,
- cycle prevention,
- location relationships,
- derived ancestry.

## Phase 5 — Search / Explore / navigation

- search,
- backlinks,
- contextual navigation,
- tabs,
- recent/pin,
- Explore filters.

## Phase 6 — Chapter editor

- loose Chapters,
- manuscript,
- notes/plan,
- Chapter links,
- context panel,
- Story usage.

## Phase 7 — safety / persistence / backups

- Archive,
- Trash,
- backup,
- restore,
- crash handling,
- integrity behavior.

## Phase 8 — integration torture

- canonical Thron/Blade/Tortuga dataset,
- full acceptance tests,
- rename/reparent/archive/crash torture.

This order is **not pre-approved architecture**. The architecture proposal may recommend a better sequence.

What matters is that implementation remains incremental and testable.

---

# 52. Development workflow after architecture approval

Do not implement the entire Milestone in one unreviewed pass.

For each implementation phase:

1. implement the scoped behavior,
2. run automated tests,
3. perform the relevant manual workflow,
4. document deviations / unresolved product questions,
5. fix regressions,
6. only then proceed.

When implementation exposes a product question, pause that behavior and escalate the decision rather than improvising silently.

---

# 53. Definition of Milestone 01 done

Milestone 01 is done when:

1. The canonical Thron / Blade / Tortuga / Arak / Temple / Chapter scenario works naturally.
2. Relationship truth is not duplicated.
3. Tortuga proves Category and Capability are separate.
4. Spatial ancestry works through derivation rather than copied parent facts.
5. A Chapter can be written with linked world context nearby.
6. Search/backlinks allow the universe to be navigated from multiple directions.
7. renames and reparenting do not break identity.
8. incomplete material is accepted without validation pressure.
9. close/reopen preserves data.
10. crash-sensitive structural operations preserve valid state.
11. Trash/backups provide credible recovery.
12. the code architecture leaves viable paths for the explicitly deferred systems.

The milestone does **not** require the application to feel finished.

It requires the foundations to feel trustworthy.

---

# 54. Architecture review checklist

Before implementation begins, reviewers should explicitly answer:

- Does the proposed domain model preserve stable identity?
- Does the relationship model store semantic truth once?
- Can fields project relationships without duplication?
- Can Relationship Instances become richer later?
- Can future multi-party knots be added without replacing the entire relationship layer?
- Are Category, Type, and Capability truly separate in the design?
- Can template evolution preserve existing authored values?
- Is Spatial primary containment distinct from ordinary relationships?
- Is recursive spatial ancestry derived?
- Can Story Units remain separate from world Entries?
- Can more Story Unit kinds be added later?
- Does persistence support atomic structural changes?
- Does autosave mean durable save?
- Can Project backups contain the whole creative workspace?
- Are derived indexes rebuildable?
- Can Projects move without internal absolute-path breakage?
- Does the architecture remain local-first?
- Are the explicit non-goals still out of implementation scope?

Any “no” requires architecture revision or an explicit product decision.

---

# 55. Closing implementation principle

The first implementation must optimize for **durable creative behavior**, not maximum feature count.

The success condition is not:

> “Worldcrafter has a lot of menus.”

It is:

> **An author can begin with Thron, discover the Singularity Blade while working, invent Tortuga as a creature that also contains cities, place Thron inside that world, move naturally between those connected ideas, write a Chapter with the relevant lore beside them, change their mind repeatedly, close the application, and trust that the universe will still be there when they return.**

That is Milestone 01.
