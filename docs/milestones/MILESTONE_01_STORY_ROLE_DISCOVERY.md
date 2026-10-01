# Milestone 01 — Story Role discovery

A Story Role describes how an Entry participates in a Chapter: a person may be
POV and Appears, while a place is Setting. It is neither a Chapter label nor an
Entry Type. The existing canonical Chapter–Entry link supports zero, one, or
several Roles. This follow-up makes that approved model easier to find and use.

## Author workflow

- Choose **Roles** beside an Entry under **In this Chapter**, without expanding
  its preview. Search available Roles and check the ones to assign. Each change
  saves through the existing Chapter command; the checkbox reflects the saved
  response. Failures stay visible in the dialog.
- **Manage available Roles** opens Chapter options. The searchable catalog shows
  assignments in the current Chapter and offers **Find uses** across the Project.
  Creating a Role makes it available; it does not silently assign it. Closing
  options returns to the assignment dialog when opened from there.
- Project Search finds Role names, including unused definitions. Selecting a Role
  shows only Chapter links carrying that Role ID. An additional text query narrows
  by linked Entry or Chapter. Archived/trashed Chapters require explicit inclusion.
  This is a list of link uses, so one Chapter can appear for multiple linked Entries.
- Story usage on an Entry continues to show its Chapters and assigned Roles.
  Removing all Roles preserves the link; Remove link is a separate action.
- Back/Forward retains the query and Role scope. Finding uses from Chapter options
  respects unsaved writing and Role drafts; closing a dialog never discards them.

## Storage and search

No authoritative schema or package migration is needed. Role definitions and
link assignments remain the existing source of truth. Search cache format 3 adds
Role definitions and stable assignment IDs. Missing, old, or invalid derived
caches use source SQL and rebuild; cache publication does not change the authored
Project revision. Both paths apply Role scope before result counts and limits.
Manuscript mentions, similarly named Roles, and unassigned definitions cannot
manufacture an assignment.

## Validation

Focused tests cover unused Role creation/search, cache invalidation after
assignment/removal, multiple Roles without duplicate links, preserved writing,
Entry scope, similar Role names, archived Chapters, foreign Role IDs, and old-cache
rebuilds. UI tests cover direct assignment, filtered checklists retaining other
assignments, creation feedback, returning from options, failed writes, exact usage
navigation and history, and protecting unapplied Role drafts. An existing autosave
test now waits for React's rendered saving state as well as the API call.

Headless Edge verification uses synthetic in-memory API responses, both themes,
and widths 1536, 1366, 1000 and 600. It checks the complete creation/assignment/
search/Chapter/Entry-backlink route, modal return, and overflow. This is not a
packaged Windows WebView2 manual run. Real Projects and backups are untouched.

Local verification on 2026-10-02 passed: **236 frontend tests** and **211 Rust
tests** (two subprocess helpers are ignored by discovery and invoked by their
parent crash tests). Typecheck, ESLint, Prettier, production build, Cargo fmt,
Clippy, cargo check and Git whitespace checks passed. No React act warnings
were reported. The existing rich-text chunk-size build warning remains.

Required gates: all frontend typecheck, tests, lint, format and build commands;
Rust fmt, tests, Clippy with warnings denied and check; repository diff whitespace
and forbidden-artifact/credential review. GitHub secret scanning and push
protection are enabled, with zero open secret alerts at verification. No local
secret-scanner CLI or repository scanner script is available. CodeQL has no configured workflow, default setup, or
installed CLI; no CodeQL pass is claimed.

## Windows manual checklist

Use a disposable Project or a copy.

1. Link an Entry to a Chapter. Open Roles beside it and assign POV plus a custom
   Role. Close/reopen the Project; verify both remain on the same link.
2. Create a Role without assigning it. Search its name and select it: the empty
   use list should explain assignment. Assign it, then search again.
3. Open Find uses in Chapter options, open a matching Chapter, and go Back.
   Confirm the Role scope and search survive. Check the Entry's Story usage too.
4. Remove every Role. Confirm the Entry remains linked and writing is unchanged.
   Archive the Chapter and verify the inactive-record search checkbox.
5. Leave a Role draft unfinished and choose Find uses. Confirm navigation asks
   how to handle the draft. Check both themes at normal Windows display scaling.

## Deferred work and timeline readiness

Role rename/retirement, Chapter tags, richer Story hierarchy and timelines are
separate work. The user's conditional timeline go-ahead does not resolve the
open model questions in Concept V0.02 section 20.1. The next timeline design must
settle Event Entry versus separate Event records, lightweight occurrences,
calendar units/epochs, uncertain dates/ranges, and links from Chapters to time.
Milestone 01 section 19.4 and Architecture V2 keep narrative reading order
independent from chronology. No Chapter rank, Role, prose mention, or relationship
label is reinterpreted as a date or event by this slice.
