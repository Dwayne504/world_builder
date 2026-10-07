# Desktop menu organization

## Scope

Requested desktop usability work: move repeated management buttons into familiar
top-level menus while preserving fast creation beside the author's work. This
build continues the Category settings branch, including the approved Timeline
foundation. It does not introduce a schema migration or change world facts.

| Menu | Purpose |
| --- | --- |
| File | Home: New/Open Project, restore a backup, application preferences. Open Project: backups, Project settings, guarded close and desktop exit. |
| Edit | Entry, Category, Type, Field, Relationship, Chapter and Timeline commands for the active context. |
| View | Workspace destinations, history, sidebar visibility and Appearance. |
| Help | Offline usage reference and information from the running backend. |

Unavailable context commands stay disabled. Existing Add actions, relationship
notes, Field inputs, and manuscript formatting stay close to their content.
Menu callbacks call existing controllers directly; they do not simulate clicks
on hidden controls. Context registrations disappear when a view unmounts.
Unfinished dialog drafts remain mounted and retain their navigation/close guards.

## Compatibility messages

The schema version describes the Project's storage layout. Opening a newer
schema or package version remains refused before editing. The boundary now gives
a plain-language next step while retaining the machine-readable error kind and
supported/found versions. About reads its information from Rust rather than
assuming the frontend bundle and backend have matching versions.

## Verification

Tests cover menu keyboard interaction and context availability, existing editor
and save guards, and reporting the actual backend's build information. Synthetic
Project tests verify that rejecting a newer schema in either the manifest or the
database preserves the manifest/database bytes and leaves no registered session.
Existing integration suites continue to cover autosave, close failures,
filesystem recovery, migrations and advisory locks.

All supplied real Projects and backups remain untouched. Browser checks use
synthetic IPC responses and are supplementary to Rust tests; they do not prove
native Windows window-manager behavior.

The Windows Rust suite passed (232 tests; three subprocess helpers are ignored
as standalone tests and exercised by their parent tests), alongside Rust
formatting, Clippy with warnings denied, and compiler checks. Headless Edge checks
passed in Storybook and Starship at 640×480, 1180×900 and 1600×900: nested menu
placement, editor commands, modal focus, backend build information, navigation,
and guarded close with a hidden unfinished creation form. Screenshots were
visually inspected for menu contrast and narrow-window placement.

Frontend verification passed: `npm run typecheck`, `npm test -- --run`
(270 tests in 25 files, no React act warnings), `npm run lint`,
`npm run format:check`, and `npm run build`. The build retains the existing
bundle-size advisory. `git diff --check` passed. The final full suite followed
corrections to an API-test cleanup callback and a Chapter controller effect
assertion; both now wait for the behavior they actually verify.

GitHub secret scanning and push protection are enabled with no open alerts at
verification time. The repository has no CodeQL workflow or local CodeQL/secret
scanner executable; no CodeQL result is claimed.

## Windows manual checklist

Use a disposable Project with an Entry, Category/Type, Chapter and Timeline.

1. In both Storybook and Starship, open each top-level menu, then nested Edit
   menus. Check readable contrast and popup placement at minimum and full width.
2. Use Tab, arrows, Enter and Escape. Confirm menu dismissal returns focus and
   opening a dialog moves focus into it. Background menus must not activate while
   a modal dialog is open.
3. Open each editor through Edit. Switch records/pages and verify the commands
   follow the active context. Confirm Add actions still support quick creation.
4. Leave an unfinished form or a failed save, then navigate or close via File.
   Confirm drafts remain available until explicitly saved or discarded.
5. Use File → Close Project to return Home; use File → Exit Worldcrafter and the
   native title-bar close to verify the same protection before app exit.
6. Open Help → About and check the backend versions. On disposable future-version
   fixtures, verify the newer-build message; do not alter real Project versions.
