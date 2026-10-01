# Milestone 01 — Project Search and Entry aliases

This implements the Search and basic alias portion of Architecture Proposal V2
slice 7 (Navigate), following the merged Chapter editor. The approved Concept
V0.02 and Milestone 01 sections 14 and 17 define these behaviors. This is not a
claim that the entire navigation slice or milestone is complete.

## Author experience

Search is a sidebar destination. It groups results as Entries, Chapters, Fields
and connections, and manuscript/other text. Within identity groups, exact names
or IDs precede exact aliases, name prefixes, alias prefixes, and other word
matches. Identity groups precede structured context and prose. Queries use
normalized word prefixes, with all query words required; punctuation is literal,
not a query language. Case and accent normalization supports finding accented
names without needing to type their accents.

Results show Category/Type context, matched alias, Field name/value/unit, or
relationship context as appropriate. Canonical relationship participants, labels,
and notes are searchable once; relationship-backed Fields do not manufacture
duplicate semantic connections. Actual Story links and their Roles are structured
matches. Manuscript, Plan, and Notes remain clearly labelled text matches, never
inferred links. Hidden and retained Field values remain searchable.

Each group initially shows at most ten results with its total count. The reader
can choose a positive whole-number limit per section or use Show more. Filtering
Fields, Relationships, or linked Chapters happens before totals and limits are
calculated. Archived/trashed records require the explicit inclusion checkbox.
Current and past semantic relationships remain searchable, with state identified.

Relationships use a compact sentence, oriented toward the participant whose name
or alias best matches the query. An exact name/alias wins over a prefix; ties keep
the canonical source first. An explicit Entry scope takes precedence. Reversing
presentation uses the inverse label; it never changes stored participants. Notes
are displayed separately, behind a disclosure, rather than displaying concatenated
index terms as prose. Opening the result retains that participant perspective.

Chapter rows start collapsed, with Roles or the matching writing area beneath
the title. Opening a row reveals a short excerpt and a link to the exact writing
area. Longer preview expands surrounding context up to 1,200 characters in a
resizable pane; it does not load the entire manuscript into the result. Titles
truncate visually with their full text available on hover and to assistive tools.

**Search this Entry** opens the same search surface scoped by stable Entry ID.
It includes that Entry's authored Fields, canonical connections, and Chapters
connected through explicit Story links, including their writing. An empty query
lists this scope; typing narrows it. Unlinked Chapters that merely mention the
Entry in prose do not become part of this scope. Search whole Project clears the
scope. Empty writing areas are omitted.

Results open an Entry, the exact canonical relationship in the relationship
browser, or the matching Chapter writing area. Back/Forward retains query,
inclusion setting, section filter, result limit, Entry scope, expanded rows and
previews, scroll/focus context, and Chapter area. Leaving
an editor uses the existing pending-save and unsaved-draft guards.

Entry settings has an **Other names** section. Adding an alias is explicit;
removing one requires confirming that specific alias. Failed writes retain the
draft, and closing settings does not discard it. Unapplied alias changes block
navigation until reviewed or explicitly discarded. Renaming an Entry does not
automatically create/delete aliases or change manuscript prose.

## Persistence and recovery

Schema 10 adds `entry_alias`: a stable alias ID, owning Entry ID, authored text,
and normalized lookup text. Uniqueness is per Entry after Unicode NFKC, lowercase,
and whitespace normalization. Different Entries may share an alias. Search uses
an additional accent-normalized representation, not a replacement of authored
text. Alias commands validate ownership and the expected Project revision, and
commit through the existing serialized Project worker.

The migration uses the existing external schema recovery snapshot and atomic
migration chain. Older supported Projects retain authored identities/content;
newer unsupported authoritative schemas remain protected by the existing opener.

`derived_index_state` tracks source revision, indexed revision, index schema
version, dirty state, and document count. Every authored operation already
advances the Project revision; a database trigger marks Search dirty in the same
transaction. A failed authored transaction rolls back both changes. Opening or
restoring a Project also marks this derived cache dirty.

The FTS5 index contains only rebuildable search documents. Cache format 2 adds
presentation metadata and explicit Entry-scope IDs; an older cache is rebuilt
without changing the authored schema or Project revision. Queries check revision,
version, document count, FTS integrity, and result decoding. Dirty, missing, or
damaged indexes fall back to authoritative SQL. A transactional rebuild then
replaces only the cache. Rebuild failure cannot fail an otherwise successful
source search or acknowledge/fail an authored save. It leaves the cache dirty so
the next query retries. Rebuilds are lazy on Search, avoiding full manuscript
reindexing on each autosave. Search and rebuild run on the Project worker; initial
search after edits can take longer than a subsequent indexed query.

Supported rich documents derive text from validated canonical content. A damaged
or newer document can still be found through its preserved text; opening it uses
the existing read-only recovery UI. Search never repairs or overwrites prose.

## Verification

All fixtures are synthetic temporary Projects. No supplied author Project or
backup is opened for writing, migrated, renamed, or copied into the repository.

Rust coverage includes exact-name/alias/prefix ranking, ID lookup, duplicate
aliases across Entries, Unicode and literal punctuation, explicit alias removal,
rename/prose preservation, Field units/choices, canonical relationship context,
Story links versus text mentions, area-specific results, inactive records,
stale revisions, source/index rollback, missing/newer/damaged index recovery,
failed rebuilds, reopening, Restore as Copy, schema-9 upgrade rollback and its
external recovery snapshot, and bounded results with 1,001 Entries. Feedback regressions cover forward,
inverse, symmetric and alias-based relationship wording; unchanged canonical
participants; notes separated from indexed terms; Entry scoping; pre-limit
filters; invalid scope IDs; and bounded short/long previews.

Frontend coverage includes query races, result grouping, escaped text, retry,
inactive inclusion, result limits, alias write failures/removal confirmation,
Back restoring query, opening the right Chapter area and exact relationship,
and preventing navigation from an unapplied alias. Feedback coverage also checks
collapsed previews, count validation, filters, relationship notes, Entry-specific
navigation, and Back retaining expanded rows. Existing editor regressions
continue covering autosave, failed writes, and native-close guards.

Headless Edge checks use synthetic in-memory IPC responses: both appearances,
desktop and narrow widths (1536, 1366, 1000, 600), alias addition, Entry round-trip,
Back restoring query, Chapter Notes selection, and flush before Search navigation.
Feedback checks additionally use 50 synthetic Chapters, section filtering,
compact relationship notes, short/long previews, chosen result counts, and Entry
search. Screenshots are inspected for readability and overflow. This does not replace a
manual packaged Windows/WebView2 test; no native OS-close run is claimed here.

Required gates: frontend typecheck/tests/lint/format/build; Rust format/tests/
Clippy with warnings denied/check; Git whitespace and forbidden-artifact review.
The existing rich-text JavaScript chunk warning remains non-blocking.

Local checks on 2026-10-01 pass: 227 frontend tests, 208 Rust tests, all required
formatting/lint/compiler/build checks, and Git whitespace checks. Two Rust
subprocess helpers are ignored by discovery and exercised by their parent crash
tests.

GitHub secret scanning and push protection are enabled with zero open secret
alerts at verification. There is no repository secret-scanner script or installed
scanner CLI; a supplemental changed-content review checks credential patterns,
personal paths, and forbidden artifacts. CodeQL default setup is `not-configured`,
with no workflow or local CLI, so no CodeQL pass is claimed.

## Windows manual checklist

Use a new disposable Project or a copy, never an original author Project.

1. Give an Entry an alias in Entry settings. Search its name, alias, and a prefix.
   Rename it, close/reopen the Project, and confirm the alias still finds it.
2. Search a number with its unit, a relationship note, and a Chapter sentence.
   Check that text mentions appear separately from actual Story links.
3. Open an Entry and go Back. Open a Notes result and confirm Notes is selected.
   Type and immediately select Search; verify the final writing is saved.
4. Start an alias and close settings without adding it. Try navigating or closing
   the Project; confirm the draft is retained until reviewed or discarded.
5. Archive a Chapter and confirm it appears only when inactive records are
   included. Back up, restore as a copy, and confirm aliases and text search work.
6. Search each participant in a directed and a symmetric relationship. Confirm
   the searched Entry leads the sentence, notes stay separate, and opening it
   preserves that perspective.
7. Search across many Chapters. Expand a row and its preview, filter to only
   Relationships or linked Chapters, and choose a result count above ten. Open a
   result and go Back; the filters and open rows should remain.
8. Use Search this Entry. Search another participant and words from a linked
   Chapter, then clear the query to browse that Entry's scope. Confirm unlinked
   prose mentions appear only in whole-Project search.
9. Check both appearances, keyboard navigation, and your normal Windows scaling.

## Explicit deferrals

Advanced Explore filters (including direct versus recursive location semantics),
search highlighting inside rich text, tabs/pins, persistent recent search history,
Tags/Statuses, new Field kinds, rich Entry descriptions, Entry lifecycle UI,
timelines/calendars, graphs, maps, publishing/export, and additional Story hierarchy
remain separate work. Location filters must not infer semantic meaning from
user-authored relationship labels.
