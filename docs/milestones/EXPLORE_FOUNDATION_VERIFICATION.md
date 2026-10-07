# Structured Explore foundation (W07a)

Implemented against the roadmap baseline at `e8267f9`. This is a read-only view
over existing Project data. It adds no database migration, stored result list,
world fact, location inference, or change to the meaning of a relationship.
All verification fixtures are synthetic; supplied Projects and backups were
not opened or modified.

## Available workflow

Open **Explore** in the sidebar or **View → Explore**. Combine a Category, an
**exact** Type, an existing capability (Base, Spatial, Event), name/alias text,
workspace state, and one current relationship clause. Independent filters use
AND. Type descendants are not included by the exact Type operator. Name/alias
matching is a case- and accent-insensitive substring match using Search's
Unicode normalization; it does not search prose or infer structural links.

For a relationship, select its definition and the result Entry's side, using
the authored forward/inverse wording. Symmetric definitions match either side.
The other Entry is optional; selecting one restricts the opposite participant.
Only relationships that are current and active participate. The result Entry's
Active / Archived / Trash / All states choice is independent of the opposite
Entry's workspace state. Unresolved participants cannot match an Entry filter.

For location questions, explicitly choose the relationship the world uses for
location and the appropriate side. **Exactly this Entry** matches that place
only. **This place and anywhere within it** includes the selected Spatial Entry
and its primary-tree descendants. Intermediate archived or trashed places do
not sever containment. Results label direct matches separately from matches via
a contained place. Moving a place changes later results without rewriting any
relationship or Entry. No relationship name is treated as a built-in location
field.

Results default to 20 per page; 10, 20, 50 and 100 are available. The backend
limits pages to 100 and streams candidates while retaining one result page.
Choosing the other Entry uses a separate bounded name/alias query (20 results)
instead of rendering every Entry. The Catalog contains only structure choices,
not all relationship instances. Category, Type and relationship selectors reuse the
searchable 20-choice batches and keep the selected ID visible. Sorting is
deterministic by name and stable ID.

Opening a result and returning with Back/Forward restores filters and paging.
Selections use stable IDs and the view rereads names after shared edits.
Missing selections, retired relationship definitions, mismatched Category/Type,
and invalid recursive targets stay visible as issues with no results; the query
never silently removes a restriction. Failed reads expose Retry, and stale
responses cannot replace a newer query. The read runs on the existing serialized
Project worker, inside one SQLite read transaction.

## Scope boundaries

This first slice has one relationship clause with an exact/contained target
operator. Multiple simultaneous relationship clauses, Type descendants,
Tags/Statuses, saved views, Story filters, arbitrary expressions, historical
queries, and graph/map views remain later slices. Filters currently survive
session navigation, not application restart. Explore is a browsing view, so its
filter controls do not create save drafts or bypass the existing authoring
navigation guards.

## Verification

Focused backend coverage checks composed filters, exact Type semantics, Unicode
aliases, directional and symmetric relationships, current/archived relationship
state, direct and recursive Spatial targets, moved ancestors, inactive places,
invalid and retired selections, input bounds, wrong/closed Project access,
unchanged authored relationship notes/revisions, reopening, and deterministic
paging across 1,007 synthetic Entries. No Explore tables are created.

Focused interface tests check composable controls, stable-ID opening, bounded
other-Entry selection, explicit direction/containment, unresolved conditions,
failed-read retry, stale response rejection, shared-edit refresh, paging, and
an App-level Entry → Back → Forward round trip retaining query and page.

Completed on 7 October 2026:

- `npm run typecheck`
- `npm test -- --run --maxWorkers=2`: **339 passed in 33 files**, no React
  `act(...)` warnings; includes the six Explore tests and workspace navigation.
- `npm run lint`
- `npm run format:check`
- `npm run build`: passed; the existing large-chunk advisory remains (855 kB
  main JavaScript bundle before compression).
- `cargo fmt --check`
- `cargo test -j 1`: **238 passed**, three deliberately ignored subprocess
  helpers exercised by their parent tests; includes six Explore integration tests.
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Rust builds used a separate temporary target directory and one build job.
There is no configured repository secret-scan or CodeQL workflow, and neither
CLI is installed on this host, so those checks are not claimed. The baseline
dependency audit reported four advisories; dependency remediation is a separate
workload, and this slice changes no dependency versions.

## Windows manual acceptance

Use a disposable Project or a copy of a Project, never source evidence.

1. Open Explore from the sidebar and View menu with keyboard and mouse. Check
   labels, wrapping, focus, and paging in Storybook and Science Fiction themes.
2. Combine Category, exact Type, capability and an alias search; clear one filter
   and confirm that the other selections remain. Try archived/trashed results.
3. Select a location relationship, side, and place. Compare exact versus within;
   move a nested place and return to Explore to check the updated results.
4. Open a result, edit its name, return, and use Forward. Check that filter state
   and page survive while displayed names update. Try a pending authoring draft
   before navigating to Explore; the existing save/discard guard must still apply.
5. Retire a selected definition from its manager and return. Confirm the visible
   issue and no widened results. Clear or replace that filter to continue.

Native desktop visual and pointer verification is not claimed by the automated
tests. Larger-world performance should be measured during W14.
