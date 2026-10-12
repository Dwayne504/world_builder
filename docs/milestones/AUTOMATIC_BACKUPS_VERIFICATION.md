# Automatic backups (W01)

Implemented against `e8267f9` under the author's D01 approval of 7 October 2026.
All fixtures are synthetic temporary Projects. Supplied Projects, backups and
real application preferences were not opened or modified.

## Behavior

Automatic backups are on by default. The native scheduler checks changed open
Projects every 15 minutes, independently of renderer timers. Sleep coalesces missed
intervals into one attempt. Empty new Projects need no snapshot; an authored
revision without a validated recovery point is eligible at the first interval.
Unchanged revisions with an intact snapshot do not create duplicate copies.

The current default Backups directory is used, with a Project ID subdirectory.
Without a configured directory, an application-owned Automatic Backups directory
under application data is used. Directory preference changes apply to subsequent
checks. Application settings and the Backups window expose an on/off preference;
the Backups window shows the last successful copy and next check. Preferences
advance to version 3, with in-memory migration from versions 1 and 2. Corrupt and
newer preferences retain the existing fail-closed repair workflow.

The newest 20 positively identified automatic snapshots per Project are retained.
A new snapshot must validate and publish before cleanup starts. Each carries a
versioned marker identifying its Project, revision and UUID, plus hashes of its
contents. Manual, safety, other-Project, unknown, modified and incomplete material
is not eligible for cleanup. An interrupted staging or cleanup remainder is
preserved for manual inspection rather than guessed to be disposable.

Snapshots reuse the existing backup format and Restore as Copy. Publication and
retention serialize with Project edits, manual snapshots and close through the
existing worker. Files are flushed before publication. Cleanup rechecks absolute
containment and rejects redirects; on Windows only known non-redirecting Cloud
Files tags are accepted. Shared snapshot validation is read-only and rejects
SQLite WAL, SHM and rollback-journal artifacts rather than ignoring newer state.

Failures remain visible without a modal or focus change. A failed cleanup reports
that the new snapshot is safe; unchanged checks do not falsely clear that warning.
Autosave and its Saved indicator remain independent. A forced close-time snapshot,
adjustable cadence/retention, cloud synchronization and arbitrary folder cleanup
are outside this slice.

## Verification

Completed on 7 October 2026:

- `npm run typecheck`, `npm run lint`, `npm run format:check`, `npm run build`.
- Full frontend suite with one worker: 350 passed. After a final stale-response
  regression was added, the focused backup hook suite passed all 8 tests and the
  static checks passed again. No final full-suite count of 351 is claimed.
- `cargo fmt --check`, `cargo test --quiet`: 250 passed, plus three deliberately
  ignored subprocess helpers exercised by their parent tests.
- `cargo clippy --all-targets -- -D warnings`, `cargo check`.
- `git diff --check` and staged whitespace check before commit.

Controllable-clock tests cover cadence, unchanged Projects, sleep/resume,
enable/disable, current directory adoption, preference failure and closed Projects.
Filesystem tests cover publication denial, cleanup denial/interruption, corrupt
and newer markers, altered content, unrelated files, junctions, preserved assets,
retention boundaries and Restore as Copy. Concurrency tests cover saves and close.
Interface tests cover status, preferences, failure notices, focus preservation,
stale Project responses and separation from autosave.

Parallel frontend runs under competing build load hit existing short test
timeouts; the complete one-worker run passed. No timeouts were increased. The
existing bundle-size advisory remains. Four baseline dependency advisories are
addressed separately in PR #31. No local secret-scan or CodeQL runner/workflow is
configured; those are not claimed as passing local checks. GitHub secret scanning
and push protection are enabled; no open secret alerts were reported when checked.

## Windows manual acceptance

Use disposable Projects and backup folders.

1. Edit a Project, minimize the app for more than 15 minutes, and confirm a new
   automatic snapshot and status after returning. Repeat without edits and check
   that no duplicate snapshot appears.
2. Disable and re-enable automatic backups; change the default backup folder and
   confirm later copies use it. Verify keyboard focus remains in the writing area
   when a background status or failure arrives.
3. Temporarily make the disposable destination unavailable. Check the visible
   warning, continued autosave, later retry and preservation of earlier copies.
4. Restore an automatic snapshot as a new Project and inspect Entries, Chapters,
   relationships and Timeline records. Confirm the source remains unchanged.
5. Check both themes, Windows scaling and long paths in the Backups window.

Real OneDrive hydration, UNC/network storage, physical disk-full or power loss,
and a real minimized 15-minute desktop session were not independently verified.
Fault injection and component tests do not establish those native scenarios.
