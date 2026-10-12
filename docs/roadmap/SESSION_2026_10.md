# Workload delivery — October 2026

This records implemented slices, not completion of the whole Concept. Supplied real Projects and backups were not modified. All listed PRs remain unmerged. The implementation commits below are fixed review references; later documentation-only commits may advance a branch.

## Delivered slices

| PR | Scope | Branch | Implementation commit | Verification |
| --- | --- | --- | --- | --- |
| [#31](https://github.com/Dwayne504/world_builder/pull/31) | Dependency audit fixes | `fix/test-tooling-security` | `5a4f26fd7eb995b21d6d28eafcccd5c6cba01ece` | 332 frontend; audit zero advisories; static checks and build. |
| [#32](https://github.com/Dwayne504/world_builder/pull/32) | W04a Type renaming | `feature/type-renaming` | `1866e04626be5fe62f8081226d8582832cd6b24f` | 339 frontend / 235 Rust. |
| [#33](https://github.com/Dwayne504/world_builder/pull/33) | W03 Entry descriptions | `feature/entry-descriptions` | `a0d858c8939089faeeafb470f1ee49ac12e8eb7c` | 353 frontend / 244 Rust. |
| [#34](https://github.com/Dwayne504/world_builder/pull/34) | W02a Windows build proof | `build/windows-build-verification` | `e460bedec152e57e946768339a082aa6b90ab7e9` | 332 frontend / 232 Rust; native release build. |
| [#35](https://github.com/Dwayne504/world_builder/pull/35) | W07a Explore | `feature/explore` | `1c7d0e0a18dbb5ac070ad27a2b92899417c5f0a9` | 339 frontend / 238 Rust. |
| [#36](https://github.com/Dwayne504/world_builder/pull/36) | W01 automatic backups | `feature/automatic-backups` | `4f4d41f488b8de2413effc01c6224be496154643` | 350 frontend full run + 8 final hook tests / 250 Rust. |
| [#37](https://github.com/Dwayne504/world_builder/pull/37) | W09a pins and recents | `feature/workspace-pins` | `99ad439916f5f0a2eb2488971a33e2bc08b6cdaa` | 367 frontend full run + 5 final hook tests / 251 Rust. |
| [#38](https://github.com/Dwayne504/world_builder/pull/38) | W10a Markdown export | `feature/manuscript-export` | `3f61d48e069432c95e24d16ae0ff9506b1f1452a` | 347 frontend / 246 Rust; independent Markdown reader. |
| [#39](https://github.com/Dwayne504/world_builder/pull/39) | W05a Rich Text Fields | `feature/rich-text-fields` | `091b3be7843b744df65899e47829ec39338438af` | 378 frontend / 261 Rust. |
| [#40](https://github.com/Dwayne504/world_builder/pull/40) | W04b Type retirement | `feature/type-retirement` | `8952feb86243973a32e3c3c16d321faa85ed5a3f` | 398 frontend + 6 final focused tests / 270 Rust |
| [#41](https://github.com/Dwayne504/world_builder/pull/41) | W05b Entry Reference Fields | `feature/entry-reference-fields` | `c526cca1a1848f717b68895bad04d94abe64010c` | 408 frontend / 278 Rust; all required static checks and build. |

Complete implementation checks mean frontend typecheck, test suite, lint, formatting and production build; Rust formatting, test suite, strict all-target Clippy and cargo check; and Git whitespace checks. Each Rust count excludes three intentionally ignored subprocess helpers exercised by parent tests. PR #31 changes frontend tooling only, so Rust was not rerun there. The existing large JavaScript chunk advisory remains.

## Dependency order

- PRs #31–36 and #38 start from the merged roadmap baseline at `e8267f9`.
- #37 builds on #33; #39 builds on #37.
- #40 Type retirement builds on #39 and includes the Type-renaming work from #32. #41 Entry References builds on #40.
- Shared menus and command registration files may need reconciliation as independent PRs land. Green CI validates each branch, not a future combined build. Run the complete checks again after integration; do not drop another slice when resolving conflicts.
- No PR has been merged by the agent. Older app builds must continue refusing newer Project schemas.

## Safety and independent verification

The Windows build procedure produced a native executable from `dd158d027cccb7bf8e9b8d1d2d3878713bc03841`, SHA-256 `59f2d982e86ba36815f9e9f66640dcf89002737de0314096f7bbdb2924b4b7b0`. The final PR #34 commit adds verification documentation after that build. This is build proof, not installer, DPI, native dialog or interactive desktop release acceptance.

PR #31 fixes the four dependency advisories found in the baseline. Feature branches based directly on that baseline do not themselves include that dependency update. GitHub secret scanning and push protection were enabled with no open alerts when inspected on 11 October. No local secret-scanner or CodeQL runner/workflow was configured, so no passing local security scan is claimed.

The Markdown export fixture was read by an independent CommonMark implementation. Reading order, Unicode, formatting, literal markup and exclusion of non-manuscript text were checked without modifying the source. Native Save As interaction remains a desktop acceptance item.

Automatic backup publication and retention were tested with synthetic faults and controlled clocks. Real OneDrive hydration, UNC storage, physical disk-full/power loss and a minimized 15-minute desktop session remain manual checks. All changed interfaces need native Windows acceptance in both themes and at the author's display scaling. This session's component tests do not establish that.

## Approved decisions and remaining work

The author approved D01 automatic backups, D03 Type retirement and D07 Markdown manuscript export. Word export remains later. W02 release publication still requires D02; Tags/Status scope D04; managed assets D05; tabs/restart D06; writing history the remaining D07 policy; Story hierarchy D08; richer dates/calendars D09; historical world state D10. Later Explore operators/saved views and native release acceptance remain separate slices. W05's basic Rich Text and Entry Reference kinds are implemented; configurable reference restrictions/cardinality, merging, conversion and inline reference-target creation are not.

## Windows acceptance checklist

Use disposable Projects or copies, never supplied originals.

1. Create, write, connect, search and rename; navigate or close during autosave. Reopen and confirm the newest writing and failed-draft recovery.
2. Exercise automatic backups with the approved interval. Check automatic retention using synthetic snapshots, including a failed destination, while manual/safety backups remain intact.
3. Fill Entry descriptions and Rich Text Fields. Detach defaults, retire definitions, change Types and restore a backup as a copy; confirm writing and identities.
4. Search with combined Explore filters, then rename a selected definition or move a Spatial ancestor and confirm current results.
5. Pin and unpin records, reopen the Project, rename a pinned record and verify shortcuts still resolve by identity.
6. Export selected manuscripts in reading order; cancel Save As once, test an existing destination, and read the output independently.
7. Review and cancel Type retirement, then retire and restore. Existing assignments, active child Types and inherited Fields must remain usable.
8. Add, reorder and follow Entry References. Rename/archive their targets, change Type defaults, reopen and restore a copy; confirm stable IDs and generic incoming Search context without new Relationships.
9. Check both themes, keyboard focus, long names, narrow desktop layouts and display scaling. A release candidate needs a fresh combined build and native acceptance.

## Exact changed-file manifests

Paths are repository-relative, grouped by the implementation PR's intended base. Shared paths appear in more than one PR.

### PR #31 — Dependency audit fixes

```text
app/package-lock.json
app/package.json
docs/milestones/TEST_TOOLING_SECURITY.md
```

### PR #32 — W04a Type renaming

```text
app/src-tauri/src/domain/lifecycle.rs
app/src-tauri/src/persistence/lifecycle.rs
app/src-tauri/tests/type_renaming.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/CategoryManager.test.tsx
app/src/CategoryManager.tsx
app/src/ProjectSearch.tsx
app/src/ProjectionFields.test.tsx
app/src/types.ts
docs/milestones/TYPE_RENAMING_VERIFICATION.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #33 — W03 Entry descriptions

```text
app/src-tauri/src/application/error.rs
app/src-tauri/src/application/service.rs
app/src-tauri/src/domain/entry_description.rs
app/src-tauri/src/domain/mod.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/persistence/entry_description.rs
app/src-tauri/src/persistence/error.rs
app/src-tauri/src/persistence/migrations.rs
app/src-tauri/src/persistence/migrations/0012_entry_descriptions.sql
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/search.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/tests/entry_descriptions.rs
app/src-tauri/tests/fields_foundation.rs
app/src-tauri/tests/relationships_foundation.rs
app/src-tauri/tests/search_foundation.rs
app/src-tauri/tests/spatial_foundation.rs
app/src-tauri/tests/story_foundation.rs
app/src-tauri/tests/templates_and_units.rs
app/src-tauri/tests/timeline_foundation.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/EntryDescriptionPanel.css
app/src/EntryDescriptionPanel.test.tsx
app/src/EntryDescriptionPanel.tsx
app/src/ProjectSearch.test.tsx
app/src/api.ts
app/src/entryDescriptionRecovery.test.ts
app/src/entryDescriptionRecovery.ts
app/src/entryDescriptionTestFixtures.ts
app/src/entryDescriptionTypes.ts
app/src/useEntryDescription.test.ts
app/src/useEntryDescription.ts
docs/milestones/ENTRY_DESCRIPTIONS_VERIFICATION.md
docs/roadmap/README.md
```

### PR #34 — W02a Windows build proof

```text
.github/workflows/windows-build.yml
docs/milestones/WINDOWS_BUILD_VERIFICATION.md
docs/roadmap/README.md
scripts/Verify-WindowsBuild.ps1
```

### PR #35 — W07a Explore

```text
app/src-tauri/src/application/service.rs
app/src-tauri/src/domain/explore.rs
app/src-tauri/src/domain/mod.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/persistence/explore.rs
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/tests/explore_foundation.rs
app/src/App.css
app/src/App.test.tsx
app/src/App.tsx
app/src/Explore.test.tsx
app/src/Explore.tsx
app/src/WorkspaceFrame.tsx
app/src/api.ts
app/src/exploreTypes.ts
app/src/workspaceHistory.ts
docs/milestones/EXPLORE_FOUNDATION_VERIFICATION.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #36 — W01 automatic backups

```text
app/src-tauri/Cargo.lock
app/src-tauri/Cargo.toml
app/src-tauri/src/application/automatic_backups.rs
app/src-tauri/src/application/mod.rs
app/src-tauri/src/application/service.rs
app/src-tauri/src/application/state.rs
app/src-tauri/src/backup_recovery/automatic.rs
app/src-tauri/src/backup_recovery/error.rs
app/src-tauri/src/backup_recovery/mod.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/preferences/mod.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/src/tauri_boundary/dto.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/AutomaticBackupPanel.css
app/src/AutomaticBackupPanel.test.tsx
app/src/AutomaticBackupPanel.tsx
app/src/CategoryManager.test.tsx
app/src/FieldMergeReview.test.tsx
app/src/api.ts
app/src/automaticBackupTestFixtures.ts
app/src/automaticBackupTypes.ts
app/src/types.ts
app/src/useAutomaticBackups.test.ts
app/src/useAutomaticBackups.ts
docs/milestones/AUTOMATIC_BACKUPS_VERIFICATION.md
docs/roadmap/DECISIONS.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #37 — W09a pins and recents

```text
app/src-tauri/src/application/service.rs
app/src-tauri/src/domain/mod.rs
app/src-tauri/src/domain/navigation.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/persistence/migrations.rs
app/src-tauri/src/persistence/migrations/0013_workspace_navigation.sql
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/navigation.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/tests/entry_descriptions.rs
app/src-tauri/tests/fields_foundation.rs
app/src-tauri/tests/relationships_foundation.rs
app/src-tauri/tests/search_foundation.rs
app/src-tauri/tests/spatial_foundation.rs
app/src-tauri/tests/story_foundation.rs
app/src-tauri/tests/templates_and_units.rs
app/src-tauri/tests/timeline_foundation.rs
app/src-tauri/tests/workspace_navigation.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/Timeline.tsx
app/src/WorkspaceFrame.tsx
app/src/WorkspaceRecords.css
app/src/WorkspaceRecords.test.tsx
app/src/WorkspaceRecords.tsx
app/src/api.ts
app/src/useWorkspaceNavigation.test.ts
app/src/useWorkspaceNavigation.ts
app/src/workspaceNavigationTestFixtures.ts
app/src/workspaceNavigationTypes.ts
docs/milestones/WORKSPACE_PINS_VERIFICATION.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #38 — W10a Markdown export

```text
app/src-tauri/Cargo.lock
app/src-tauri/Cargo.toml
app/src-tauri/src/application/error.rs
app/src-tauri/src/application/export_publication.rs
app/src-tauri/src/application/manuscript_export.rs
app/src-tauri/src/application/mod.rs
app/src-tauri/src/domain/manuscript_export.rs
app/src-tauri/src/domain/mod.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/persistence/manuscript_export.rs
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/tests/manuscript_export.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/ManuscriptExportDialog.css
app/src/ManuscriptExportDialog.test.tsx
app/src/ManuscriptExportDialog.tsx
app/src/api.ts
app/src/manuscriptExportTestFixtures.ts
app/src/manuscriptExportTypes.ts
docs/milestones/MANUSCRIPT_EXPORT_VERIFICATION.md
docs/roadmap/DECISIONS.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #39 — W05a Rich Text Fields

```text
app/src-tauri/src/domain/fields.rs
app/src-tauri/src/persistence/fields.rs
app/src-tauri/src/persistence/migrations.rs
app/src-tauri/src/persistence/migrations/0014_rich_text_fields.sql
app/src-tauri/src/persistence/search.rs
app/src-tauri/tests/relationships_foundation.rs
app/src-tauri/tests/rich_text_fields.rs
app/src-tauri/tests/search_foundation.rs
app/src-tauri/tests/templates_and_units.rs
app/src/CategoryManager.test.tsx
app/src/CategoryManager.tsx
app/src/DeleteEntryFieldDialog.tsx
app/src/EntryDescriptionPanel.css
app/src/EntryDescriptionPanel.tsx
app/src/EntryDocumentPanel.tsx
app/src/EntryFieldsPanel.tsx
app/src/FieldManagerTables.tsx
app/src/FieldMergeReview.tsx
app/src/RichFieldPanel.test.tsx
app/src/RichFieldPanel.tsx
app/src/fieldLabels.ts
app/src/types.ts
app/src/useEntryDescription.ts
app/src/useEntryDocument.ts
app/src/useEntryFields.ts
docs/milestones/RICH_TEXT_FIELDS_VERIFICATION.md
docs/roadmap/WORKLOADS.md
```

### PR #40 — W04b Type retirement

Base: `feature/rich-text-fields`.

```text
app/src-tauri/src/application/service.rs
app/src-tauri/src/domain/lifecycle.rs
app/src-tauri/src/domain/structure.rs
app/src-tauri/src/lib.rs
app/src-tauri/src/persistence/lifecycle.rs
app/src-tauri/src/persistence/migrations.rs
app/src-tauri/src/persistence/migrations/0015_type_retirement.sql
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/spatial.rs
app/src-tauri/src/persistence/type_retirement.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src-tauri/src/tauri_boundary/dto.rs
app/src-tauri/tests/entry_descriptions.rs
app/src-tauri/tests/fields_foundation.rs
app/src-tauri/tests/fixtures/remove_type_retirement.sql
app/src-tauri/tests/relationships_foundation.rs
app/src-tauri/tests/search_foundation.rs
app/src-tauri/tests/spatial_foundation.rs
app/src-tauri/tests/story_foundation.rs
app/src-tauri/tests/templates_and_units.rs
app/src-tauri/tests/timeline_foundation.rs
app/src-tauri/tests/type_renaming.rs
app/src-tauri/tests/type_retirement.rs
app/src-tauri/tests/workspace_navigation.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/CategoryManager.test.tsx
app/src/CategoryManager.tsx
app/src/ProjectSearch.tsx
app/src/ProjectionFields.test.tsx
app/src/TypeRetirementReview.test.tsx
app/src/TypeRetirementReview.tsx
app/src/api.ts
app/src/typePresentation.ts
app/src/typeRetirementTestFixtures.ts
app/src/types.ts
docs/milestones/TYPE_RENAMING_VERIFICATION.md
docs/milestones/TYPE_RETIREMENT_VERIFICATION.md
docs/roadmap/DECISIONS.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### PR #41 — W05b Entry Reference Fields

Base: `feature/type-retirement`.

```text
app/src-tauri/src/application/service.rs
app/src-tauri/src/domain/fields.rs
app/src-tauri/src/persistence/entry_references.rs
app/src-tauri/src/persistence/fields.rs
app/src-tauri/src/persistence/migrations.rs
app/src-tauri/src/persistence/migrations/0016_entry_references.sql
app/src-tauri/src/persistence/mod.rs
app/src-tauri/src/persistence/search.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/tests/entry_descriptions.rs
app/src-tauri/tests/entry_reference_fields.rs
app/src-tauri/tests/fields_foundation.rs
app/src-tauri/tests/fixtures/remove_entry_references.sql
app/src-tauri/tests/relationships_foundation.rs
app/src-tauri/tests/search_foundation.rs
app/src-tauri/tests/spatial_foundation.rs
app/src-tauri/tests/story_foundation.rs
app/src-tauri/tests/templates_and_units.rs
app/src-tauri/tests/timeline_foundation.rs
app/src-tauri/tests/type_retirement.rs
app/src-tauri/tests/workspace_navigation.rs
app/src/CategoryManager.test.tsx
app/src/CategoryManager.tsx
app/src/DeleteEntryFieldDialog.tsx
app/src/EntryFieldsPanel.test.tsx
app/src/EntryFieldsPanel.tsx
app/src/EntryReferenceValue.test.tsx
app/src/EntryReferenceValue.tsx
app/src/FieldMergeReview.tsx
app/src/fieldLabels.ts
app/src/types.ts
app/src/useEntryFields.ts
docs/milestones/ENTRY_REFERENCE_FIELDS_VERIFICATION.md
docs/roadmap/README.md
docs/roadmap/WORKLOADS.md
```

### Documentation follow-up on PR #38

After the fixed implementation commit above, a documentation-only commit adds this delivery report and updates `docs/roadmap/DECISIONS.md` and `docs/roadmap/README.md`. It does not change the exported manuscript behavior.
