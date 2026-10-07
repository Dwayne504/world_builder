# W05a Rich Text Fields — verification

## Scope

Rich Text is now an optional Field kind in Entry creation and Category/Type defaults. It uses the same bounded, versioned document AST and writing toolbar as Chapters and Entry descriptions. Existing Short Text and Description Fields keep their identities and kinds; no content is automatically converted.

Schema 14 adds the typed `field_value.document_id` reference to the existing `rich_document` store. Each document belongs to exactly one Entry and Field. Blank defaults allocate neither values nor documents. Documents with unsupported versions, invalid AST/JSON, or a non-current migration state remain preserved and read-only. Their original content is available for copying; editing, clearing, and deletion are blocked, while Hide remains available.

Authored Rich Text survives default detachment, Category/Type changes, Field-definition retirement, hiding, reopen, and backup/restore. Deletion remains local to the Entry and requires the existing automatic recovery backup. A validated empty editor document clears the value. Formatting-only documents are retained. Rich Text duplicate merging is explicitly blocked in this slice; neither document is chosen or discarded automatically.

Search derives faithful text from validated documents and reports Rich Text under Fields, not as a duplicate Entry description. Cached/extracted text is never an independent source of authored writing or semantic links.

## Save and recovery contracts

- Save acknowledgements keep the mounted editor, focus, selection and undo history.
- Every save checks both the Project revision and the document revision. Rich document revisions use the next Project revision, so clearing and recreating a value cannot reuse a stale editor's revision.
- Typing during a save remains pending until its own acknowledgement. A failed write retains its draft and offers retry or explicit comparison with saved writing.
- Emergency drafts have separate Project/Entry/Field keys. Unsupported recovery formats remain visible for copying and cannot be applied automatically.
- Entry navigation and close guards include all Rich Field controllers. Passive waits drain existing requests; they do not start new writes before relationship saves finish.
- A successful Rich save cannot refresh over a dirty, saving or failed simple Field draft, including edits begun while a companion refresh is in flight.

## Automated evidence

All data fixtures are generated under disposable temporary directories. No supplied Project or backup was opened or modified.

Focused backend tests cover optional creation, Category/Type defaults, detachment, Field-definition retirement, hiding, reopen/restore, faithful search without duplicate description hits, AST/schema validation, stale and concurrent writers, wrong-Project/inactive writes, injected transaction failure, clear/recreate revision protection, database ownership guards, blocked merges, local deletion and restoration from its recovery backup. A populated schema-13 migration test verifies rollback, foreign-key preservation, choice values, existing unsupported documents and workspace pins.

Focused frontend tests cover initial content loading, autosave focus/undo, empty defaults, Category/Type default creation, passive wait ordering, clear/retype, dirty and failed companion Field drafts, explicit recovery review, unsupported content, Hide versus blocked Delete, and close/controller state. The existing Entry description tests continue to exercise the extracted shared writing surface.

Full integrated checks passed on Windows:

- Frontend: `npm run typecheck`, `npm run lint`, `npm run format:check`, `npm test -- --run --maxWorkers=1` (378 tests across 38 files), and `npm run build`.
- Rust: `cargo fmt --all -- --check`, `cargo test -j 1` (261 tests passed; three subprocess helpers are excluded from ordinary discovery and exercised by their parent tests), `cargo clippy -j 1 --all-targets -- -D warnings`, and `cargo check -j 1`.
- Repository: `git diff --check` and the complete diff against the parent feature branch passed.

The parent workspace navigation UI is integrated in this branch; its checks ran together with this slice. The frontend build retains the existing large-chunk advisory (865.84 kB JavaScript, 262.06 kB compressed). No runtime or dependency changes were made to address that separate optimization.

## Deferred and manual verification

Entry Reference Fields, automatic conversions, attachments, additional formatting, and Rich Text duplicate merging remain deferred. No new product decision was needed for this slice.

No native desktop visual/keyboard smoke test was performed for this slice. Automated frontend checks use jsdom; backend checks run native Windows Rust code. Linux CI and a final Windows desktop smoke test remain independent checks. The repository supplies no secret-scan or CodeQL command/workflow, and no corresponding CLI was available. Dependency audit remediation is tracked separately; this slice does not change dependencies.

### Windows smoke test (disposable Project)

1. Add an empty Rich Text Field to an Entry and as a Category/Type default. Opening it should not create a value until writing begins.
2. Type formatted paragraphs and lists; continue typing and using Undo through autosave. Check the Saved indicator, reopen, and search for distinctive prose.
3. Change the Entry's Type and detach/retire the default. Confirm authored writing stays present. Hide and show it, then delete locally and restore its backup as a copy.
4. Switch between two Rich Fields, simple Fields and relationship notes; navigate or close during saving. Confirm the newest text survives and a failed save keeps the Entry open with a recoverable draft.
5. Clear a Rich value, type again, then reopen. Check both appearances, narrow windows, keyboard toolbar access, long prose scrolling, and focus visibility.
