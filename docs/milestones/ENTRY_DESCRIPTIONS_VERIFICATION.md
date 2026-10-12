# Rich Entry descriptions — W03

An Entry now has an optional rich writing area independent of its structured Fields. Add description opens the existing Chapter rich editor, with the same supported formatting. Description text participates in Search without creating semantic connections. Existing Fields called Description are preserved.

## Persistence and save guarantees

- Migration 12 extends the existing versioned document store to Entry descriptions. Chapter content and record identities remain unchanged. Ownership constraints reject missing or mismatched records.
- Writes compare both the Project revision and the loaded description revision in one transaction. A concurrent description edit cannot be overwritten merely because another acknowledged operation advanced the Project revision.
- Autosave drains newer typing before navigation or close. Failed writes retain the draft; retries keep their failed baseline until the author explicitly reviews the saved version.
- A best-effort local recovery buffer is scoped by Project and Entry. Unsupported or damaged saved/recovery data is preserved and exposed for copying rather than silently rewritten.
- Descriptions survive rename, Category/Type changes, Archive/Trash/Restore, reopen and Restore as Copy. Inactive Entries and newer document formats cannot be edited.

## Verification

Verified on Windows on 7 October 2026 using synthetic temporary Projects only:

- Frontend typecheck, lint, format check and production build passed.
- Full frontend suite: **353 tests passed** with `npm test -- --run --maxWorkers=2`. Focused coverage includes editor identity, focus and undo across acknowledgements, delayed writes, failures, recovery, Search routing, companion edits and native-close callbacks.
- Rust: **244 tests passed**, with three subprocess helper tests intentionally ignored as standalone tests and exercised by their parent tests. `cargo fmt --check`, strict all-target Clippy, and `cargo check` passed.
- Backend regressions cover transactional migration/rollback, preserved Chapter payloads, malformed/newer documents, stale document revisions, ownership, lifecycle, backup/Restore as Copy and derived search rebuilding.
- Git diff checks passed. No configured local secret scanner or CodeQL runner was available; these are not claimed as passing scans.

The first full frontend run overlapped several heavy builds and hit existing five-second test timeouts. With bounded workers and those builds staggered, the unchanged suite passed without React `act` warnings. Test timeouts were not increased. The production bundle still emits its existing size advisory.

## Windows manual checks remaining

Component tests exercise native-close callbacks but do not establish actual WebView2 behavior. No interactive desktop walkthrough was performed for this slice.

1. Create a disposable Project and Entry, add a formatted description, type continuously through autosave, then close/reopen. Check focus, selection and undo.
2. Search for a word found only in that description and open its Entry; rename and move the Entry, then repeat.
3. Leave a description named Field in place and confirm both remain independent.
4. Back up and Restore as Copy; verify description formatting and links. Use disposable fixtures for older/newer schema checks.
5. Check both themes, keyboard access, a narrow desktop window and Windows scaling. Simulate a failed write only in disposable data and verify the draft stays recoverable.

Images, embedded records, new formatting nodes, passage links and collaborative editing remain deferred. This is a complete W03 implementation awaiting PR review and native acceptance, not a declaration that the entire application is release-ready.
