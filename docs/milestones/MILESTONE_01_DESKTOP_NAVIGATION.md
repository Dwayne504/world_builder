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
  Small counts show all Entries and each Category's total, independent of the
  current Type filter. The Category section also collapses separately. Its **+**
  buttons start an Entry in that Category through the same navigation/save guard;
  unfinished creation drafts remain protected.
- Back and Forward, including Alt+Left / Alt+Right, retain the browsing filter,
  collapsed Category groups, list scroll, and Entry-link focus where practical.
  Following a relationship creates a history visit. Revisiting an Entry reads
  committed content rather than replaying a stored copy of its values.
- Back to Entries opens the current Category/Type list. Creating an Entry from a
  filtered Category seeds that Category/Type without altering an unfinished draft.
- Project menu contains Categories, Backups, Project settings, Appearance, and
  Close Project at the bottom. Its panel uses the workspace palette, independently
  of the leather/metal header, and opening it does not move the menu label.
- Relationships in the sidebar opens a read-only Project-wide card browser.
  It shows five canonical connections initially, in creation order, with five
  more per Show more. Entry selections include either endpoint (any selected
  Entry); the definition and lifecycle filters intersect that selection. The
  default includes current and past connections. Definition names, notes,
  ended/retired states, unavailable endpoints, and cardinality warnings remain
  visible. Open an active participant to edit in its existing Entry editor.
  Back restores selected IDs, lifecycle filter, amount shown, scroll and focus;
  returning fetches committed data. Filtering never rewrites a connection.
- The initial desktop window is 1180 × 800, with a 640 × 480 minimum. The sidebar
  reflows in a narrow window; Entry columns depend on available content width.

There are no new unresolved product decisions in this slice.

## Timeline boundary

Concept V0.02 sections 6.3/6.5, 8.9, 11.3, 17.3 and 20.1 explicitly defer
Timelines, fictional calendars and the Event Capability. Narrative Chapter order
is separate from world chronology and must never be silently reordered by time.
The Concept leaves Event Entries versus separate event records, lightweight
occurrences, calendar units/epochs, uncertain dates/ranges, and state-at-time
queries for later product decisions.

Preparation in this slice is limited to a separate workspace-page identity in
session navigation and a collapsible Category section. A future approved
Timeline can be another navigation destination; it need not be a Category or a
second copy of relationship facts. There is no Timeline placeholder, date schema,
calendar arithmetic, event model, or graph implementation here. Existing
relationship identities and explicit ended state remain unchanged; they are not
reinterpreted as a calendar/history system.

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

The Project relationship query runs on the same serialized database worker as
Entry reads and mutations. It reads each semantic instance once, with the same
participants and project-wide soft-cardinality warnings as the Entry projection.
It does not aggregate duplicated inverse views or write records. Counts derive
from the Entry list; no count or relationship-browser tables are persisted.

No Project schema or preference schema migration is introduced. Supplied real
Projects/backups were not used for these tests; generated disposable Projects
and a separate verification app identifier isolated the native checks.

## Verification

Windows automated checks:

- `npm run typecheck`
- `npm test -- --run` — 161 tests
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` — 157 tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Focused coverage includes concurrent Recent Projects updates, retention rollback,
duplicate names, rename/restart, unavailable and relocated packages, wrong
identity before normal or interrupted-manifest open, corrupt/newer recent stores,
explicit orphan-lock recovery UI, shortcut removal acknowledgment, exact Type
filtering, history branch truncation, fresh Entry reads, scroll/filter restoration,
failed destinations, unfinished drafts, in-flight save failures, and visible
native-window close failures. Follow-up coverage adds sidebar counts/collapse,
Category + creation and save guards, Project menu placement, relationship
pagination, filter intersection/either endpoint, duplicate names with stable IDs,
symmetric/self instances, conflicts, missing participants, read failure/retry,
late responses, Project isolation and history/filter restoration.

Earlier verification of the original navigation changes used a Windows Tauri
executable with its production frontend bundle and real WebView2/IPC/backend
(debug profile, no installer). Verified:
one-click Recent Project open, Category browsing, Human filtering, editing a
numeric value through the whole row, autosave, following a relationship, keyboard
Back/Forward, preserved filter, and restored scroll/focus in a 32-Entry list.
Restart retained the three Recent Projects and the edited value. Sidebar
collapse and the layout at the initial window size and maximized were inspected.
The final build's Alt+F4 check exited the native window and released the Project
lock. The separate verification app was closed afterward.

The September 30 follow-up used automated tests only, as requested: no visible
app or desktop input control. A headless Edge run used the production frontend
bundle with synthetic in-memory IPC responses. Both themes passed menu position
and overflow checks at widths 640, 1180 and 1600 px; opening moved the label 0 px.
Menu text contrast measured 6.34:1 in Storybook and 7.97:1 in Starship. The same
run covered sidebar collapse/counts, Category + preselection, five-card expansion,
lifecycle filtering, participant navigation, Back with restored filter/focus,
and Close Project. Screenshots of both card layouts were inspected. These are
frontend checks, not new native WebView2/IPC end-to-end verification; the Rust
suite separately covers the new Project query. Initial sandbox-only filesystem
failures were rerun successfully with the required test access.

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
5. In Storybook and Starship, open Project menu: text should be readable, its
   label should stay still, and Close Project should be the last action. Check
   the counts, collapse Categories, and use + beside a Category; confirm the
   new Entry starts in that Category and an unfinished draft stays protected.
6. Open Relationships with more than five connections. Check Show more, select
   Entries on either side, combine a definition and state filter, open a card's
   Entry, then go Back. Confirm the filter and position return and each
   connection appears once. Check notes, ended connections and conflict warnings.
7. Close a clean Project with Alt+F4 or the title-bar X. Confirm the window exits
   and the same Project reopens without a leftover-lock warning. Check unsaved
   and failed-save close prompts separately.

Not independently verified for this follow-up: native WebView2 rendering and IPC
of the new Relationships page, actual minimum-size desktop resizing, installer/
signing, another machine or OS, every display-scaling/accessibility configuration,
forced process termination during a Recent Projects transaction, and every
filesystem/device failure. Transaction rollback and corrupt/newer-store behavior
are covered by automated tests.

## Deferred

Full tabs, pinned/recent Entries, session restoration across app restarts,
Search/Explore, relationship graph, full ribbon toolbar, custom colors, further
cursor-light refinement, Story, Spatial, Timeline, and unrelated milestone work.
