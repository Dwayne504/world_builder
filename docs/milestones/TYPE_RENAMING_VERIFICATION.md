# Type renaming — W04a

This slice implements the approved rename part of W04. Type retirement, restoration, deletion and reparenting remain outside this change; W04b still needs the D03 lifecycle decision.

## Behavior

- Open **Edit → Type → Types and defaults** from a Category or Entry, or **Edit → Category → Categories and defaults** from anywhere in the Project. Select the Type's defaults, then choose **Rename Type**.
- The dialog starts with the current name. A dismissed draft, including an empty name, remains available through **Continue manager draft**. A blank name cannot be submitted. Cancel discards only the draft.
- The selected Type stays selected after its name changes. Duplicate names are disambiguated in the manager by their stable IDs; renaming never merges definitions with matching names.
- Current Entry labels, Type selectors, inherited-default source labels and open Search results refresh in the same session. Search keeps its query and view state.
- A failed write retains the draft. A stale revision can be explicitly reloaded and retried. An acknowledged rename is not offered as an uncommitted draft if a subsequent refresh fails.

## Persistence and scope

The existing Project-scoped, serialized `apply_structure` command now accepts `rename_type`. The backend validates the expected global revision, the selected Type's existence in that Project, and the same definition-name rules used on creation: trim surrounding whitespace, reject blank names, and limit to 200 Unicode characters.

One transaction updates only the Type's name, timestamp and revision plus the Project revision. Category membership, parent/child IDs, Entry assignments and revisions, Field values and availability, capabilities, Relationships and Story links are unchanged. Search remains a rebuildable projection validated against the Project revision. No schema migration, dependency or backup-format change is required.

## Automated coverage

Synthetic temporary Projects cover Unicode and duplicate-name renames; missing, wrong-Project, stale, blank and overlong edits; inherited Field and capability defaults; retained authored values, Relationships and Story links; immediate search-label refresh; reopen; and backup/Restore as Copy with internal IDs preserved.

Frontend coverage includes the selected Type, duplicate-name selection, empty/dismissed drafts, target-switch prevention, stale reload/retry, pending acknowledgements, refresh failure after commit, current Entry and selector refresh, and refreshing Search without changing its query.

All verification commands below passed on Windows: 339 frontend tests and 235 Rust tests. Three ignored Rust subprocess helpers are exercised by their parent integration tests.

- `npm run typecheck`
- `npm test -- --run --maxWorkers=2`
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test -j 1`
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

The default parallel frontend run reached several existing timing failures under concurrent build load. Verification limits worker concurrency without increasing test timeouts. An existing projection-field test now waits for the asynchronous controller state rather than assuming its effect runs before the error text appears. The initial parallel Rust compile encountered missing cached artifacts; the single-job retry completed. The production bundle retains its existing size advisory.

There is no configured repository secret-scan or CodeQL workflow and no local Gitleaks, TruffleHog or CodeQL executable. Those scans were not run for this slice. No real Projects or backups were opened or modified.

## Windows manual checklist

Use a disposable Project or a copy. Never test writes against supplied originals.

1. Create a Category, parent and child Types, an inherited Number Field and a typed Entry. Give the Field a value and link the Entry to a Chapter and another Entry.
2. Open **Edit → Type → Types and defaults**, select the Type, and rename it. Check the Entry heading, Category Type filter, Entry settings selector, inherited defaults and existing links.
3. Search for the Entry. While Search remains open, rename its Type using the global Category manager. Confirm the result's Type label updates without changing the search.
4. Clear the rename input, close the child dialog, and continue the draft. Confirm it remains blank and cannot be saved. Cancel it and confirm the saved name remains.
5. Close/reopen the Project, then make a manual backup and Restore as Copy. Check the Type name, inherited defaults, values and links in the copy.

Interactive Windows WebView2 behavior, display scaling and manual accessibility checks have not been independently exercised for this slice. Automated tests do not replace that checklist.
