# W05b Entry Reference Fields — verification

## Scope and boundaries

An optional Entry Reference Field can hold zero, one or several ordered pointers to Entries in the same Project. This implements ordinary “See also” or research links described in Milestone 01 sections 9 and 10 and Architecture V2 sections 4 and 12. It creates no semantic Relationships or inverse relationship labels. Configurable single-only cardinality and target restrictions are not introduced in this slice; existing values are not coerced to a narrower shape.

Entry-local Fields and Category/Type defaults use the same definition and value machinery. Empty defaults allocate no reference rows. Selected Entries can be opened, reordered, removed, searched and paged; candidate and selected lists render at most ten rows per page. Inactive targets remain labelled and navigable, while new selections require active Entries. Self references and cycles are valid because these links are not Spatial containment.

Schema 16 adds an ordered, typed reference store separate from scalar values and Relationships. Project-local IDs and database foreign keys determine identity; labels are presentation only. Retained reference rows keep their IDs through reordering. Unsupported newer schemas continue to be refused. Existing unresolved snapshots stay visible and cannot be silently repaired by creating a similarly named Entry. This slice does not add permanent Entry deletion or a snapshot-repair workflow.

## Persistence and editing

- Saves check both the Project revision and the exact prior ordered reference IDs. Reloading the Project revision cannot silently authorize overwriting a conflicting selection.
- Missing, wrong-Project, wrong-kind and duplicate targets are rejected atomically. Existing inactive or unresolved tokens can be retained.
- Pending save controls wait for acknowledgement. Failed selections remain in the existing Field controller; refreshing saved Fields preserves those drafts. “Use saved references” is an explicit choice.
- Hide, template detachment, Type reclassification, Type retirement and Field retirement preserve authored references. Entry-local deletion removes only that Entry's references after the existing automatic recovery backup.
- Derived Search results expose generic incoming references from the same source rows. Search indexes are rebuildable and never create independently authored backlinks.
- Entry Reference duplicate merging is explicitly blocked; both selections remain intact.

## Automated evidence

All fixtures are synthetic temporary Projects. No supplied Project or backup was opened for writing or copied into the repository.

Eight focused Rust tests cover empty/single/multiple values, stable order and IDs, rename, self/cyclic links, inactive targets, stale clear/recreate and concurrent writes, invalid and foreign targets, injected transaction rollback, unresolved snapshots, SQL guards, defaults/inheritance/reclassification, hide/detach/retire/reopen, local deletion and Restore as Copy, search and cache fallback, and recoverable schema-15 migration failure and retry. Older synthetic migration fixtures explicitly remove the new table when constructing their older test schema.

Frontend tests exercise bounded thousand-Entry lists, search, selection ordering, navigation, inactive/unresolved display, loading failures and retry, disabled mutations during saving, retained failed drafts and stale expectations, empty creation, autosave acknowledgement, and Category/Type defaults.

Complete checks passed on Windows on 11 October 2026:

- Frontend: `npm test -- --run` (408 tests across 40 files), `npm run typecheck`, `npm run lint`, `npm run format:check`, and `npm run build`.
- Rust: `cargo fmt --check`, `cargo test` (278 passed; three intentionally ignored subprocess helpers are exercised by parent tests), `cargo clippy --all-targets -- -D warnings`, and `cargo check`.
- Repository: `git diff --check` and review of the complete change against Type retirement.

The existing JavaScript chunk-size advisory remains (877.27 kB, 265.04 kB compressed). Dependency and bundle optimization are separate work.

## Deferred and independent acceptance

Configurable cardinality and target restrictions, inline target creation, non-Entry targets, conversion from other Field kinds, unresolved-target repair, permanent deletion and reference merging remain deferred. These are separate slices; this does not mark every Field metadata requirement complete.

No native desktop interaction was performed for this slice. React tests use jsdom; Rust tests execute on Windows. Native WebView2 appearance, focus, keyboard behavior and display scaling still require acceptance. The repository has no configured local secret-scan or CodeQL command/workflow; no passing run of either is claimed. Dependency remediation is tracked in PR #31 rather than silently mixed into this branch.

## Windows checklist

Use a disposable Project or copy.

1. Create “See also” locally and as a Type default. Leave one empty; add one and then several targets to another.
2. Search and page a long candidate list, reorder targets, follow a link and return. Confirm the newest selection survives autosave and reopen.
3. Rename, archive and restore a target. Its existing reference should keep its identity and show the current label/state.
4. Detach a default, change or retire a Type, hide/show the Field, and confirm values survive.
5. Delete the Field from one Entry; restore the automatic recovery backup as a copy and confirm the original reference IDs and targets.
6. Open an Entry's search context and find its incoming reference without a new semantic Relationship.
7. Test keyboard access, both themes, long names and Windows scaling. A failed save must retain the selection and keep navigation/close guarded.
