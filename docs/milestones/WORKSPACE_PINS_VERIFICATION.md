# Pins and recent records (W09a)

This slice builds on the Entry descriptions branch (PR #33). Schema 13 adds
Project-local navigation metadata after the description migration at schema 12.
All fixtures are synthetic; supplied Projects and backups remain untouched.

## Available workflow

Open an Entry, Chapter or Timeline occurrence and choose **View → Pin current
record**. The sidebar's collapsible Pinned and Recent sections show five records
per page, with search when needed. **View → Pinned and recent…** opens the manager,
with ten records per page, unpinning, a recent-list limit and Clear recent records.
The default recent limit is 20; the interface offers 10, 20 and 50. Pins retain
the order in which the author pinned them. Explicit drag reordering is deferred.

Recents are recorded only after a record successfully opens. Opening a shortcut
uses the existing navigation and pending-save guards. Names and Archive/Trash
labels are resolved from current records rather than copied into the metadata.
Missing targets stay visible but cannot be opened, and can be unpinned. Clearing
recents or removing a pin never changes or deletes authored material.

Metadata writes use the serialized Project worker and an independent interface
queue. They do not advance the authored global revision or alter the Saved badge.
A shortcut failure does not block ordinary access to Project content. Failed
explicit actions remain retryable even if a subsequent passive refresh fails.
Warnings remain reachable with the sidebar hidden. Pins and recents survive
reopening and Restore as Copy, within the new copy's own Project identity.

There are no tabs, automatic last-Project reopening or editor-position restoration
in this slice. Those remain gated by D06. The existing lock, migration review,
newer-schema refusal and emergency draft recovery mechanisms are unchanged.

## Verification

Completed backend checks on 7 October 2026:

- `cargo fmt --check`
- `cargo test`: 251 passed; three deliberately ignored subprocess helpers are
  exercised by their parent tests.
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`

Seven focused integration tests cover Project isolation, identity/kind validation,
stable names through rename and lifecycle changes, bounded recents, independent
pins, invalid-limit rollback, close/reopen, Restore as Copy, missing targets,
concurrent metadata versus description writes, and transactional schema-12
migration failure/retry. Existing older-schema fixtures were updated to remove
the new tables when constructing a deliberately older database.

Frontend checks passed: `npm run typecheck`, `npm run lint`,
`npm run format:check`, `npm run build`, and the full one-worker test suite
(367 tests in 37 files). After the final correction preserving a failed explicit
action across failed passive visits, all five hook tests and the static/build
checks passed again. The full-suite count is from before that final correction.
No React `act(...)` warnings occurred. Earlier full runs under contention hit two
existing five-second App-test timeouts; both passed isolated and in the final full
run without increased timeouts. Repository staged and unstaged diff checks pass.

Interface coverage includes bounded collections and long labels, failed-read
recovery, retry intent, stale-response rejection, Entry/Chapter/occurrence visits,
live rename refresh and failed-save navigation protection. The existing bundle
size advisory remains. Baseline dependency advisories are addressed separately
in PR #31. No local secret-scanner or CodeQL runner/workflow is configured; these
are not claimed as passing local checks.

## Windows manual acceptance

Use a disposable Project or a copy.

1. Open and pin an Entry, Chapter and Timeline occurrence. Check their labels and
   ordering in the sidebar and manager, using both keyboard and pointer.
2. Rename and Archive/Trash a pinned record. Confirm the shortcut updates, opens
   the intended stable record, and offers the existing recovery workflow.
3. Open more records than the selected recent limit. Check paging, search and
   clearing recents; pins and authored content must remain intact.
4. Try a shortcut with unsaved or failed writing. Confirm the normal save/review
   guard runs and the draft survives a cancelled navigation.
5. Close and reopen, then Restore as Copy. Check metadata stays Project-scoped.
   Verify long names, narrow layout, both themes and Windows scaling.

Native desktop interaction and display acceptance were not independently tested.
Component and backend tests do not establish those scenarios.
