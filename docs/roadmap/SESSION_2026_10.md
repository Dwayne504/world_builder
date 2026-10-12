# Workload delivery — October 2026

This records implemented slices, not completion of the whole Concept. Supplied real Projects and backups were not modified. The author explicitly authorized merging ready work on 12 October. The implementation commits below remain fixed review references; use the integration record for the newer combined heads and checks.

## Integration — 12 October 2026

PR #31 was already merged by the author at `fa328f4c1ad6166b31cff483f9955ce87a49df97`. The remaining work was integrated in a separate clean checkout with ordinary merge commits. No history was rewritten, no branch was force-pushed, and prior worktrees and local edits were preserved. Shared menus, API commands, save controllers, tests and Windows dependency features were reconciled rather than choosing one branch's version wholesale.

| Order | PR | Verified integrated head | Main merge commit |
| --- | --- | --- | --- |
| 1 | #34 Windows build | `aff4f015d76cb7fe6cb81a899a8cf16e3b785916` | `d81d41b015cd51e3effc1ec2e5411dfc000ca7bd` |
| 2 | #32 Type renaming | `0c05806b671f39ab0ef0b4333ad7dd9156367fef` | `6a7c8dd7b15f475daf1709823772c9869306649a` |
| 3 | #33 Entry descriptions | `929f412de1ec68eff73d212d992dcc460499f371` | `c890324458d02f052fc897063a7ff084dd54d3b8` |
| 4 | #37 Pins and recents | `447c278ecc5c44d4986092e453d4c2dc66c94c7e` | `e1d88bbd5896a8855b7c1956e4a7be9a35b6dc4a` |
| 5 | #39 Rich Text Fields | `479cee6ca14a7b453331e1be938fdf91c913068b` | `f62060cf03398f6509f6b8267fdaf3179ba035f4` |
| 6 | #40 Type retirement | `69924e7b53913d1ecb1acc1395a860713f390c52` | `9513c0d9c74626757a90e447f1bf5ce9f16b7d99` |
| 7 | #41 Entry References | `18d10d433440d9bf5fb2529162a70b6b781975f4` | `5ca9ddebdd1faefdf6ad7069bbb628dde0aac46f` |
| 8 | #35 Explore | `ec0566c07fb3c09d9a37536be248e870153dcce2` | `cb7ec078adfb14fbdf220fdeab5398ee22dd0706` |
| 9 | #36 Automatic backups | `4142cc48c54c470b4dab0b4eb781971c6a688c80` | `fa58cbe8a93549cf9c308142accbf79c9b52d21f` |
| 10 | #38 Markdown export and this handoff | Code at `a3011d0df19e5713e19dfe5d238b1922d912bbc3`; this PR adds documentation only afterward | Final merge gate: current head CI and unchanged prospective merge content |

Before each merge, the exact PR head, base, non-draft status and successful checks were verified. The prospective merge tree also had to equal the tested head's tree. Stacked PRs were retargeted to `main` as their dependencies landed. Fetch `main` after #38 merges; older feature checkouts are preserved historical work, not the launch target for all features.

The combined code passed [Linux CI](https://github.com/Dwayne504/world_builder/actions/runs/38217426368) and a [clean Windows build](https://github.com/Dwayne504/world_builder/actions/runs/38218199087). Windows verified all 449 frontend tests, all 316 Rust tests, frontend typecheck/lint/format/build, Rust fmt/Clippy/check, Git whitespace and clean-source checks, and the native release executable. Three subprocess test helpers remain intentionally ignored as standalone tests and are invoked by parent tests. The workflow publishes the executable and its checksum/commit report as `worldcrafter-windows-a3011d0df19e5713e19dfe5d238b1922d912bbc3`.

Local Windows Rust tests and strict checks passed too. Local frontend runs encountered five-second time limits under load; a later run was interrupted by computer suspension for nearly three hours. Focused reruns passed, but no clean full local frontend pass is claimed. The complete clean Windows CI run is the independent frontend evidence. Test assertions and timeouts were not relaxed. The existing frontend bundle-size advisory remains.

Fresh npm audit reported zero vulnerabilities. GitHub secret scanning and push protection were enabled, with zero open alerts on 12 October. CodeQL and a local secret scanner were not configured; no successful run is claimed. Native interactive UI, display scaling, dialogs, long-running backup sessions, installers and updates still require separate acceptance. This is a verified build, not a completed release acceptance cycle.

Integration resolutions are recorded in the merge commits above. The final handoff edits only `docs/roadmap/README.md`, `docs/roadmap/SESSION_START.md` and this file. No product decision was added. W14a desktop acceptance is the next useful workload; new gated features still require their recorded decisions.

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

## Original implementation dependency order

- PRs #31–36 and #38 start from the merged roadmap baseline at `e8267f9`.
- #37 builds on #33; #39 builds on #37.
- #40 Type retirement builds on #39 and includes the Type-renaming work from #32. #41 Entry References builds on #40.
- Shared menus and command registration files may need reconciliation as independent PRs land. Green CI validates each branch, not a future combined build. Run the complete checks again after integration; do not drop another slice when resolving conflicts.
- This was the original pre-integration dependency graph; the 12 October record supersedes its merge status. Older app builds must continue refusing newer Project schemas.

## Safety and independent verification

The Windows build procedure produced a native executable from `dd158d027cccb7bf8e9b8d1d2d3878713bc03841`, SHA-256 `59f2d982e86ba36815f9e9f66640dcf89002737de0314096f7bbdb2924b4b7b0`. The final PR #34 commit adds verification documentation after that build. This is build proof, not installer, DPI, native dialog or interactive desktop release acceptance.

PR #31 fixes the four dependency advisories found in the baseline. The original implementation commits based directly on that baseline did not include the update; all integrated heads now do. GitHub secret scanning and push protection were enabled with no open alerts when inspected on 11 October and again during integration on 12 October. No local secret-scanner or CodeQL runner/workflow was configured, so no passing local security scan is claimed.

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
