# Milestone 01 — Desktop navigation foundation

This slice implements the approved desktop navigation work following the focused
Entry workspace. It follows Concept V0.02, Milestone 01 navigation requirements,
and Architecture Proposal V2 section 14. It does not complete the whole Navigation
or Search/Explore milestone.

## Author workflow

- Home lists the last three Projects by name. Click a name to open it. Options
  exposes the saved location, Locate folder, and Remove from recent list.
- The collapsible sidebar browses All Entries or one Category. A Category view
  offers All Types, No Type, or an exact Type. Filtering changes the view only;
  it does not change Entry assignments or include descendant Types implicitly.
- Back and Forward, including Alt+Left / Alt+Right, retain the browsing filter,
  collapsed Category groups, list scroll, and Entry-link focus where practical.
  Following a relationship creates a history visit. Revisiting an Entry reads
  committed content rather than replaying a stored copy of its values.
- Back to Entries opens the current Category/Type list. Creating an Entry from a
  filtered Category seeds that Category/Type without altering an unfinished draft.
- Project menu contains Categories, Backups, Project settings, and Appearance.
  Close Project remains inside Project settings.
- The initial desktop window is 1180 × 800, with a 640 × 480 minimum. The sidebar
  reflows in a narrow window; Entry columns depend on available content width.

There are no new unresolved product decisions in this slice.

## Persistence and safety

`application_home::RecentProjectsStore` owns `recent-projects.sqlite` in the OS
app-config directory, outside every Project package. Schema version 1 uses SQLite
transactions, full synchronization, a busy timeout, and an immediate write
transaction for update plus retention. Concurrent app processes cannot overwrite
each other's list from a stale in-memory snapshot. Corrupt or unsupported newer
stores fail visibly and are preserved; ordinary Browse/Open remains available.

Project ID is identity, including when names collide. A successful create, open,
restore-as-copy, or rename records the current name, canonical package location,
and access timestamp. A monotonic access sequence orders updates even if the
system clock moves backwards. Retention evicts the oldest shortcuts beyond three.
Missing paths otherwise remain repairable until removed. Removing a shortcut
never deletes a Project. Listing checks directory availability without opening
packages or taking Project locks.

Locate folder verifies the expected Project ID using read-only database
preflight **before** manifest recovery, lock acquisition, or schema migration.
A different Project is rejected without promoting its interrupted manifest
publication files. The correct Project retains existing lock-recovery and
migration behavior. An active OS lock remains authoritative; orphan recovery is
still explicit. A recent-list failure is a separate warning and cannot turn a
successful Project commit into an apparent save failure.

History is bounded to 100 session locations and holds stable IDs plus presentation
state, not authored data. Fetch failures leave the current location/history
unchanged. Navigation waits for in-flight writes; failed saves retain their
drafts and do not silently retry because another editor finished first.
Unapplied structural/definition drafts require explicit resolution. The content
is disabled during asynchronous navigation so new edits cannot arrive after the
save decision.

Native verification also exposed a pre-existing missing Tauri window-close
capability and a race between a second close event and listener disposal. The
main window now has the specific `core:window:allow-destroy` permission. Only
after the save guard and backend session shutdown succeed, it awaits native
window destruction directly. If closing the window fails after session shutdown,
Home shows that failure rather than losing it with the unmounted editor.

No Project schema or preference schema migration is introduced. Supplied real
Projects/backups were not used for these tests; generated disposable Projects
and a separate verification app identifier isolated the native checks.

## Verification

Windows automated checks:

- `npm run typecheck`
- `npm test -- --run` — 153 tests
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` — 155 tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Focused coverage includes concurrent Recent Projects updates, retention rollback,
duplicate names, rename/restart, unavailable and relocated packages, wrong
identity before normal or interrupted-manifest open, corrupt/newer recent stores,
explicit orphan-lock recovery UI, shortcut removal acknowledgment, exact Type
filtering, history branch truncation, fresh Entry reads, scroll/filter restoration,
failed destinations, unfinished drafts, in-flight save failures, and visible
native-window close failures.

Native verification uses a Windows Tauri executable with its production frontend
bundle and real WebView2/IPC/backend (debug profile, no installer). Verified:
one-click Recent Project open, Category browsing, Human filtering, editing a
numeric value through the whole row, autosave, following a relationship, keyboard
Back/Forward, preserved filter, and restored scroll/focus in a 32-Entry list.
Restart retained the three Recent Projects and the edited value. Sidebar
collapse and the layout at the initial window size and maximized were inspected.
The final build's Alt+F4 check exited the native window and released the Project
lock. The separate verification app was closed afterward.

GitHub secret scanning and push protection are enabled; the repository reported
zero open secret alerts at review time. No local secret-scanner executable or
script is configured. CodeQL is not configured, has no available analysis, and
has no local CLI here; CodeQL was not run. A local diff review checks for secrets,
personal paths, supplied packages, machine settings, and generated output before
commit.

## Windows follow-up checklist

1. Open three disposable Projects, restart, and open one by its Recent name.
   Rename it and reopen it; the shortcut should use its new name. Create a fourth
   Project and confirm only the last three remain.
2. Browse Categories, switch All Types / one Type / No Type, open an Entry,
   follow a relationship, then use Back/Forward. Check filter, scroll, and focus.
3. Type a value, then navigate during autosave. Confirm successful changes survive
   reopening; a failed save must retain its draft and offer explicit resolution.
4. Move a closed disposable package. Locate folder should repair its shortcut;
   choosing a different Project must report the mismatch. Remove from recent list
   should remove only the shortcut.
5. Verify Project menu and collapsed sidebar at your normal Windows display
   scaling and at a narrow window size. Check that text and dialogs remain usable.
6. Close a clean Project with Alt+F4 or the title-bar X. Confirm the window exits
   and the same Project reopens without a leftover-lock warning. Check unsaved
   and failed-save close prompts separately.

Not independently verified: minimum-size/narrow-window layout (the automation
resize attempt did not change the window), installer/signing, another machine
or OS, every display-scaling/accessibility configuration, forced process termination during a
Recent Projects transaction, and every filesystem/device failure. Transaction
rollback and corrupt/newer-store behavior are covered by automated tests.

## Deferred

Full tabs, pinned/recent Entries, session restoration across app restarts,
Search/Explore, relationship graph, full ribbon toolbar, custom colors, further
cursor-light refinement, Story, Spatial, Timeline, and unrelated milestone work.
