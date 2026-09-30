# Milestone 01: Spatial foundation

Implements the approved V2 “Tortuga” structural slice, following Concept V0.02
Capabilities/Spatial and Milestone 01 sections 12–13 and 42. No new product
semantics are introduced.

## Author workflow

- Any Entry can enable Spatial without changing its Category or Type. Base is
  implicit and always present.
- Category and Type Spatial defaults are combined with ancestor-Type defaults
  when creating an Entry, including relationship/Field quick-created Entries.
  The resulting feature set belongs to that Entry. Later default or classification
  changes never alter existing feature sets.
- An Entry has at most one structural parent. Roots are valid. Arrange places
  supports changing the parent, moving to the top level, creating a child, and
  explicitly moving an existing Spatial Entry with its subtree.
- Breadcrumbs, direct children and searchable descendants are derived from parent
  links. Long lists are bounded with further results available. Parent selection
  excludes self and descendants; the backend also rejects cycles atomically.
- Related places displays the actual relationship label and the related Spatial
  Entry's structural path. This does not assert that ownership, birthplace, or an
  arbitrary relationship means Current Location, nor create ancestor relationships.
- Spatial removal requires first moving the Entry to the top level and moving its
  children elsewhere. The Entry, Fields and Relationships remain intact.
- Child drafts survive dismissal. Native close/navigation waits for in-flight
  writes and protects unfinished or failed drafts. Explicit reload retains drafts.

## Storage and publication

Schema 8 adds fixed Base/Spatial definitions, Category/Type defaults, Entry-owned
capabilities and indexed `spatial_node` parent links, with timestamps and revisions.
Existing Entries gain Base only. No authored classification or values are changed.
The existing upgrade path locks the Project and validates an external schema-7
recovery snapshot before an atomic migration and recoverable manifest publication.

Creation triggers materialize capabilities for every Entry creation path. SQLite
foreign keys and recursive cycle triggers protect structural integrity. Base
cannot be removed. Spatial commands use the existing serialized database worker,
`BEGIN IMMEDIATE`, and an expected Project revision. A move updates one node, its
revision/timestamp, and the Project revision. Descendants, Entry classification,
and Relationships are untouched. Reads inside the transaction produce the response
before commit; failed operations do not return a success acknowledgement.

There is no ancestry cache or index yet to invalidate: paths are derived from the
current adjacency snapshot on every read. The snapshot uses Project-scoped Entry
IDs. Archived/Trashed nodes remain resolvable with markers; new moves require
active nodes. No name-based repair or location-role inference is performed.

## Acceptance and automated verification

- Tortuga → Northern Shell → Arak → Temple; Thron has one direct location
  relationship to Temple. Move Arak to Floating Continent and back. Temple's parent,
  revision and timestamp, Thron's Entry, relationship ID, target and note remain
  unchanged; ancestry follows the new path.
- Self/descendant cycles, inactive or non-Spatial parents, foreign IDs and stale
  writes fail without changing the snapshot. Competing moves have one winner.
- Category/ancestor-Type defaults form a deduplicated union only at creation;
  quick creation follows the same rule. Reclassification preserves features.
- Dependent feature removal is refused. Removing Base and removing a referenced
  Spatial parent through SQL are also rejected.
- Injected failures roll back child creation and moves. Subprocess tests exit
  without worker shutdown/checkpoint: committed moves survive and uncommitted
  parent edits roll back. The ignored helper test is invoked by that parent test.
- A failing schema-7 upgrade remains schema 7 and retryable. Recovery retains the
  original Entry; a successful retry yields Base-only existing Entries.
- Backups and reopen retain the tree and stable relationship identities.
- Frontend tests cover breadcrumbs, direct/recursive search, bounded lists,
  parent selection, error/retry, hidden drafts, feature defaults, derived
  relationship context, and native close waiting for a child commit.

Required checks: frontend typecheck, complete tests, lint, format check and build;
Rust formatting, complete tests, strict all-target Clippy and check; Git whitespace
check. Verification results and native observations are recorded in the PR.

Local verification on Windows: 193 frontend tests and 184 Rust tests passed, plus
all required checks above. The subprocess-only helper is marked ignored for direct
test enumeration and is exercised by the passing recovery test. The native Tauri
build used bundled production assets and isolated preferences. A disposable
Project confirmed feature activation, child creation, breadcrumb navigation,
moving a child to the top level and back, and clean close/reopen. No supplied real
Project or backup was opened for writing.

GitHub secret scanning and push protection are enabled with no open alerts at
verification time. A local scan of changed files found no credential/private-path
patterns. No repository CodeQL workflow or local CodeQL CLI is available; the
code-scanning API returned 404, so CodeQL is not independently verified.

## Windows checklist (disposable Project)

1. Enable Spatial on a Creature such as Tortuga; confirm classification stays.
2. Create Northern Shell, Arak and Temple as nested children; open each through the
   child list and breadcrumbs. Switch between direct children and all descendants.
3. Move Arak to another Spatial root. Confirm Temple follows and keeps its Fields
   and Relationships. Move it back. Self/descendant choices must be unavailable.
4. Link a non-Spatial Entry directly to Temple. Related places must show the
   relationship's label and Temple's current hierarchy without extra connections.
5. Enable a Category/Type Spatial default. New Entries inherit it, old ones do not;
   removing the default leaves existing Spatial Entries intact.
6. Dismiss an unfinished child form, then try navigation and OS close. Recover or
   cancel the draft. Close/reopen cleanly and confirm the saved tree is unchanged.

## Explicitly deferred

Maps, coordinates, routes/graph editing, a dedicated world-tree workspace, and
location-specific “directly here / anywhere within” Explore filters. Those filters
must distinguish relationship semantics through an approved definition contract;
they cannot infer Current Location from an authored name. Following a moving
container's own situational-location relationship is also deferred. Current
location/adjacency can already be authored as ordinary Relationships, separately
from containment. Timelines, Story/Chapters and other milestone features are not
part of this change.
