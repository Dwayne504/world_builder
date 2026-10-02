# Timeline foundation verification

This is the first dated slice of the [approved Timeline roadmap](../architecture/TIMELINE_FOUNDATION_PROPOSAL.md).
It builds on the Story Role discovery work. It does not complete the advanced
calendar or historical-query roadmap.

## Implemented

- Optional unnamed, undated occurrences; exact fictional dates when wanted.
- One Project calendar with named months, fixed day counts and an optional era
  label. Years can be negative or zero. Calendar setup is explicit.
- Chronological browsing, ten results initially, search, Entry/Chapter filters,
  Archive, Trash and Restore.
- Lightweight moments and optional full Event Entries with canonical Fields and
  Relationships. Context links do not create semantic world Relationships.
- Entry/Chapter backlinks and Project Search targets by stable occurrence ID.
- Autosave that retains focus and failed drafts, drains newer edits before
  navigation/desktop close, and does not silently rebase stale writes.
- Schema 11 migration, calendar payload validation and transactional publication.
  Corrupt/newer calendars block Timeline edits without blocking search of stored
  occurrence titles, notes and links.

## Automated checks

Run on Windows, using synthetic Projects in temporary directories only:

| Check | Result |
| --- | --- |
| `npm run typecheck` | Passed |
| `npm test -- --run` | 247 tests passed across 22 files; no React `act(...)` warnings |
| `npm run lint` | Passed |
| `npm run format:check` | Passed |
| `npm run build` | Passed; Vite reports the existing large-bundle advisory |
| `cargo fmt --check` | Passed |
| `cargo test` | 223 passed; three ignored subprocess helpers are exercised by their parent crash tests |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo check` | Passed |
| `git diff --check` | Passed |

Focused coverage includes date validity/sorting, same-day and undated records,
calendar mutation guards including archived/trashed dates, corrupt/newer payloads,
canonical Event references, Project-scoped links, transaction rollback, competing
revision writes, abrupt exit after acknowledgement, migration failure/retry with
external recovery, Restore as Copy identity preservation, derived search scoping,
backlinks, autosave focus, pending/failed native-close handling and draft retention.

Background Edge automation used the actual frontend with synthetic mocked desktop
commands. Both themes were visually reviewed; 1536, 1366, 1000 and 600 pixel widths
had no horizontal overflow. Checks covered filtering, autosave/focus, calendar
renaming and structure guards, linked Entry navigation, Back to the selected
occurrence, backlinks and Project Search navigation. This did not control the
author's desktop or open supplied Projects.

GitHub secret scanning and push protection are enabled; the repository had no open
secret alerts at verification. No standalone repository secret-scan command or
local scanner was available. CodeQL default setup is not configured, and there is
no CodeQL workflow or local CLI; CodeQL analysis was not run. Pull-request CI
provides the separate Linux build/test result.

## Windows manual checklist

The packaged Tauri/WebView2 interface has not been independently exercised for
this slice. Use a new disposable Project or a disposable copy, then:

1. Open Timeline, create a moment without a title or date, write notes, navigate
   away and back, and close/reopen the app. Confirm acknowledged notes remain.
2. Set up a small fictional calendar. Date several moments, including year zero,
   a negative year, a shared day and an undated item; verify their ordering.
3. Link an Entry and a Chapter; choose a full Event page. Follow links both ways
   and use Back. Confirm Chapter reading order and manuscript text are unchanged.
4. Search and filter occurrences, then Archive/Trash/Restore one. Confirm its
   date, notes and links survive. Rename a month; existing dates should show its
   new name while day counts and month counts remain protected.
5. Type notes and immediately navigate or close the window. Confirm pending
   saves finish. If a save fails, the visible draft must remain until retry or
   explicit discard.

Deferred: multiple calendars/conversions, time of day, approximate or partial
dates, ranges/durations, leap rules, recurrence, multiple independent timelines,
historical calculations, automatic relationship/state transitions and graph views.
