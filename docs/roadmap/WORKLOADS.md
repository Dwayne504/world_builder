# Workload packets

Each packet defines a user outcome, bounded slices, code starting points and acceptance tests. Use the [session contract](SESSION_START.md) for all implementation and delivery checks. Decision IDs refer to the [decision register](DECISIONS.md); a design slice may finish with a proposal rather than executable code.

## W01 Automatic backups

**Outcome:** an author can recover recent work without remembering to create backups manually. Existing manual backups and safety snapshots remain recoverable.

**Slices:** W01a proposes and obtains D01 policy approval. W01b implements the serialized scheduler, snapshot classification, preference validation and minimal status UI. W01c implements approved retention with failure injection and restore verification. W01b must not introduce unbounded automatic storage as a finished feature; keep it internal until the complete policy is verified.

**Start in:** `app/src-tauri/src/backup_recovery/`, `persistence/snapshot.rs`, `persistence/worker.rs`, `preferences/`, `application/service.rs`, and the existing backup UI in `app/src/App.tsx`. Paths without an `app/` prefix in backend maps are relative to `app/src-tauri/src/`.

**Acceptance tests:**

- Use a controllable clock to verify cadence, unchanged Projects, sleep/resume and coalesced missed intervals without real-time waits.
- Concurrent editing, manual backup and close cannot overlap unsafe snapshot publication or produce a partially acknowledged save.
- A failed or interrupted backup leaves the live Project and previous valid backups intact; failure is visible without stealing typing focus.
- Retention considers only positively identified automatic snapshots for the current Project. Manual, safety, unknown and other-Project material is preserved.
- Prune only after successful validation; exercise disk-full, permission denial, corrupt metadata, partially published snapshots and interrupted cleanup.
- Restore as Copy preserves internal identities, Chapters, Timeline, definitions and links while assigning a new Project identity. Include assets as support evolves.

**Deferred:** cloud backup, external sync engines, arbitrary folder cleanup and changing the existing backup format. Architecture V2's initial packaging proposal does not authorize replacing the current validated snapshot format during this slice.

**Session selection:** W01a first; W01b and W01c only after D01. Branch suggestion: `feature/automatic-backups`.

## W02 Reliable Windows builds and releases

**Outcome:** the author can start a known build without guessing which development checkout or database version it supports.

**Slices:** W02a verifies a reproducible Windows build and documents launch/install procedures with visible version/build information. W02b prepares release automation and an installer acceptance procedure. Publishing or adding an updater waits for D02.

**Start in:** `app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml`, `app/package.json`, `app/src/api.ts`, `app/src/AppHelp.tsx`, `.github/workflows/` and existing build-info tests. Keep version sources consistent; do not commit binaries.

**Acceptance tests:** build from a clean checkout; launch the packaged desktop executable against a synthetic Project; verify native pickers, lock ownership, close/reopen and save paths. Exercise upgrade from a supported fixture, rejected newer schemas, paths with spaces/non-ASCII characters and a user account without development tools. Confirm uninstall/upgrade does not delete authored Projects or backups. Check installer behavior in an isolated Windows environment; a successful bundle command is not proof of installation.

**Deferred:** silent upgrades, public publication, signing purchases and release tagging without explicit authorization. No schema downgrade to make an old executable open a newer Project.

**Session selection:** W02a is ready. Branch suggestion: `build/windows-release-verification`.

## W03 Rich Entry descriptions

**Outcome:** an Entry has an optional writing area for its biography, lore or description, independent of short structured Fields.

**Slices:** W03a adds a persisted rich description using the existing validated Chapter document machinery and autosave behavior. W03b integrates description text into search with migration, recovery and backup coverage. Ship the complete searchable workflow before marking W03 done.

**Start in:** `app/src/RichTextEditor.tsx`, `writingSchema.ts`, `ChapterEditor.tsx`, `App.tsx`; backend `domain/structure.rs`, `domain/`, `persistence/story.rs`, `persistence/search.rs` and migrations. Extract shared document behavior where useful; do not fork the Chapter editor into an unrelated implementation.

**Acceptance tests:** an empty Entry needs no description; formatted content survives reopen, rename, Category/Type changes, Archive/Trash/Restore and Restore as Copy. Autosave preserves selection and focus. Failed/stale writes retain the unsaved draft. Older fixtures migrate transactionally; malformed/newer document payloads fail safely. Search finds authored description text and refreshes after edits without manufacturing relationship links.

**Deferred:** images, embedded live records, passage links, collaborative editing and new formatting nodes beyond the supported document schema. Preserve any existing Field called “Description”; do not move or delete its content automatically.

**Session selection:** W03a is ready under Architecture V2's Entry description model. Branch suggestion: `feature/entry-descriptions`.

## W04 Type management

**Outcome:** an author can maintain Types from the Category manager without recreating Entries or losing their values.

**Slices:** W04a exposes Type rename with stable identity and clear current selection. W04b adds reviewed retirement/restoration after D03. Consider permanent deletion only in a separate, explicitly approved workload.

**Start in:** `app/src/CategoryManager.tsx`, existing Category settings, `types.ts`; backend `domain/structure.rs`, `application/service.rs`, definition persistence and existing template derivation tests.

**Acceptance tests:** renaming updates selectors, current Entry labels and search results while IDs, Field values, Chapter links and relationships remain unchanged. Duplicate display names cannot cause identity confusion. Validate empty/invalid drafts consistently with existing definitions. For retirement, test assigned Entries, descendants, inherited defaults, reactivation and attempts at new assignment against the approved policy. Stale review results must be revalidated before mutation.

**Deferred:** deep inheritance editors, automatic conversion between incompatible Categories and destructive definition cleanup.

**Session selection:** W04a is ready; W04b is gated by D03. Branch suggestion: `feature/type-management`.

## W05 Remaining foundational Field kinds

**Outcome:** authors can write formatted Field content and make ordinary references to other Entries without misusing relationship Fields.

**Slices:** W05a implements Rich Text Fields using the shared document schema. W05b implements an Entry Reference Field under the existing concept's non-semantic reference rules. Verify the specified single/multiple value shape before implementing; any unresolved cardinality choice is a decision gate, not a reason to borrow relationship semantics.

**Start in:** `app/src/EntryFieldsPanel.tsx`, `ProjectionFieldValue.tsx`, `useEntryFields.ts`, shared rich editor; backend `domain/fields.rs`, `persistence/fields.rs`, search and migrations. W03's shared document work is recommended first.

**Acceptance tests:** all new values survive detach, Type change, retirement, reopen and restore. Reference values use Project-scoped IDs; renamed, unnamed and inactive targets remain intelligible. References are navigable and discoverable as references without creating semantic relationship facts or inverse labels. Validate wrong-Project and missing targets. Rich text is searchable, AST-validated and autosaved without focus loss. Old Field kinds and reviewed duplicate merging continue to work; incompatible kinds cannot be merged silently.

**Deferred:** computed Fields, formulas, automatic unit conversion, new relationship semantics and migration of arbitrary short text into references.

**Session selection:** one Field kind per PR. Branch suggestions: `feature/rich-text-fields`, then `feature/entry-reference-fields`.

## W06 Tags and creative Status Systems

**Outcome:** authors can organize material with loose labels and independent progress/canon systems without changing its Type or lifecycle.

**Slices:** W06a implements Tags for the record kinds approved in D04. W06b implements named Status Systems and their values. Add search/filter integration with each slice rather than shipping definitions that cannot be found or used.

**Start in:** record identity and definition persistence, search, `app/src/types.ts`, `storyTypes.ts`, Entry/Chapter editors and contextual menus. Reuse stable identity, revision and lifecycle infrastructure.

**Acceptance tests:** rename keeps assignments; Unicode and same-name records remain distinguishable; Tag assignment has no structural effect. An item can be Drafting in Writing and Canon in Canon simultaneously. An unset Status is valid. System/value edits preserve existing assignments or require a reviewed mapping. Archive/Trash and current/past relationship state are independent. Unsupported record kinds are rejected at the backend, and all metadata survives backups and Restore as Copy.

**Deferred:** claim-level certainty, per-character beliefs, automatic workflow transitions and arbitrary status-driven behavior.

**Session selection:** D04 first, then W06a. Branch suggestion: `feature/tags` followed by a separate Status PR.

## W07 Explore and saved views

**Outcome:** authors can answer questions such as “Which Characters are anywhere inside Tortuga?” without creating manual lists.

**Slices:** W07a delivers composable Category, Type, capability, relationship and location filters with explicit exact-versus-recursive location. W07b adds Tags/Statuses when W06 exists and saves reusable live queries by stable IDs. Narrow the first filter set if needed, but label unsupported operators rather than silently changing their meaning.

**Start in:** `app/src/ProjectSearch.tsx`, `WorkspaceFrame.tsx`, existing relationship and Spatial browsers; backend `persistence/search.rs`, `spatial.rs`, `relationships.rs` and query boundaries.

**Acceptance tests:** combine filters using the documented query semantics; test direct location versus containment-derived location, moved ancestors, missing locations and inactive records. Renames preserve saved filters; missing/retired definitions produce visible unresolved conditions rather than widening results silently. A saved view recomputes after edits and never duplicates canonical records. Large synthetic collections are paginated or bounded, keyboard-accessible and responsive.

**Deferred:** arbitrary SQL, user formulas, graph visualization and historical “at time” queries until W13.

**Session selection:** W07a is ready without Tags/Statuses; read Milestone 01 Explore requirements before choosing operators. Branch suggestion: `feature/explore`.

## W08 Managed images and attachments

**Outcome:** a Project can contain its own images and supporting files and remain portable.

**Slices:** W08a specifies import/display/removal behavior under D05 and proves safe managed-file publication. W08b exposes attachment and image controls with complete backup/restore coverage. Rich-document embedding follows separately if it requires new document nodes.

**Start in:** package and backup services, document schemas, persistence worker and native file selection. Coordinate filesystem staging with database publication; define recovery for each interrupted boundary.

**Acceptance tests:** import from a disposable source, then move/delete that disposable source and reopen the Project successfully. Test duplicate names, reserved Windows names, large/unsupported files, failed copies, interruption between file and database publication and portable internal paths. Path traversal or references outside the managed asset area must not be accepted as owned assets. Restore as Copy reproduces attachments. Detaching one reference cannot destroy another reference or content recoverable from Trash.

**Deferred:** remote embeds, external linked files, maps, image editing and automatic garbage collection without a separately reviewed ownership policy.

**Session selection:** D05 and W01 integration first. Branch suggestion: `feature/managed-assets`.

## W09 Workspace continuity

**Outcome:** authors can quickly return to frequently used material and resume a writing session without losing their place.

**Slices:** W09a adds author-controlled pins and bounded recent records. W09b adds record tabs and restart restoration after D06. Keep session metadata separate from authored records and from emergency draft recovery.

**Start in:** `app/src/workspaceHistory.ts`, `WorkspaceFrame.tsx`, `DesktopMenu.tsx`, mutation/close guards, backend application state and Project-local UI state storage.

**Acceptance tests:** pins and recents are scoped to the correct Project and use stable record IDs. Renames, Archive/Trash, restored copies and missing records behave predictably. Back/Forward retains filters. Opening/closing/restoring tabs waits for pending saves and preserves failed drafts. Restoring session metadata never bypasses a live lock, migration review or newer-schema refusal. Corrupt/stale UI state cannot prevent access to valid Project content.

**Deferred:** pop-out windows, cross-Project tabs, synchronization across machines and automatic reopening behavior beyond D06.

**Session selection:** W09a can start from the existing navigation specification. Branch suggestion: `feature/workspace-pins`.

## W10 Manuscript export and writing recovery

**Outcome:** authors can take a selected manuscript out of Worldcrafter and understand how to recover earlier writing.

**Slices:** W10a adds one approved export format and scope preview after D07. W10b adds further formats only as separately reviewed slices. W10c designs and implements actual document revision history only after its retention and restore policy is approved; keep it separate from export.

**Start in:** Chapter ordering/document services, `app/src/ChapterLibrary.tsx`, `ChapterEditor.tsx`, `chapterRecovery.ts`, native destination selection and atomic publication utilities.

**Acceptance tests:** export selected Chapters in reading order with Unicode, supported formatting and unnamed Chapters. Verify inclusion/exclusion of Plan, Notes and Summary against D07. Cancelled or failed exports leave an existing destination intact; overwrites require an explicit choice. Reopen the produced file in a suitable independent reader. Export never changes canonical prose or poses as a Project backup. For history, restore creates a recoverable new revision and cannot silently overwrite concurrent writing.

**Deferred:** publishing platforms, print layout, alternate story variants, passage links and EPUB until separately selected. Do not make ordinary export depend on finishing W11 hierarchy.

**Status:** D07's Markdown export scope is approved and W10a is implemented on `feature/manuscript-export`. See [verification](../milestones/MANUSCRIPT_EXPORT_VERIFICATION.md). Word/EPUB and writing-history policy remain separate work.

## W11 Story hierarchy and passage links

**Outcome:** larger works can be organized beyond a flat Chapter collection without forcing small projects into a hierarchy.

**Slices:** W11a prepares an approved hierarchy model and example workflows under D08. W11b implements just the selected first increment with migration and ordering. W11c separately specifies links from selected manuscript passages to Entries, preserving authored visible text and stable target identity.

**Start in:** Concept Story sections, Milestone 01 Chapter constraints, `domain/` Story types, `persistence/story.rs`, Chapter library/editor and document schema. Reuse existing stable Story Unit identities.

**Acceptance tests:** loose Chapters remain usable; moves preserve content, links and reading order under the approved rules. Prevent cycles and cross-Project references. Archive/Trash/Restore and export traversal follow reviewed parent/child rules. A large fixture with hundreds of units stays navigable. Passage-link rename changes the target label where appropriate but never rewrites manuscript wording; editing/deleting linked text preserves valid document structure and undo behavior.

**Deferred:** arbitrary configurable Story Unit kinds, shared units across multiple Books, storyboards, alternate versions and automatic fictional chronology from reading order.

**Session selection:** design only until D08 is settled. Branch suggestion: `docs/story-hierarchy-design` before implementation.

## W12 Advanced Timeline dates

**Outcome:** the Timeline can express what the author actually knows, including incomplete or uncertain dates, without inventing precision.

**Slices:** W12a produces a temporal model proposal and truth-table examples for D09. W12b implements the approved precision/range subset. W12c separately designs time of day, calendar evolution and multiple-calendar conversion; do not bundle all time features into one migration.

**Start in:** [approved Timeline foundation](../architecture/TIMELINE_FOUNDATION_PROPOSAL.md), `app/src/Timeline.tsx`, `TimelineRail.tsx`, `timelineTypes.ts`; backend `domain/`, `persistence/timeline.rs`, calendar validation and migrations.

**Acceptance tests:** keep exact existing dates, negative years, year zero, same-day and undated behavior valid. Define and test sort/display semantics for every approved precision and range, including overlap and unknown boundaries. The rail must expose uncertainty visibly rather than position an approximate date as an exact fact. Calendar conversion requires preview, backup, transactional publication and deterministic retry; failed conversion preserves original dates. Archive/Trash dates are included in impact review.

**Deferred:** derived age, historical ownership/location, recurrence and automatic state transitions. No calendar conversion by label matching, operating-system timestamps or assuming Earth month lengths.

**Session selection:** W12a is a design workload, not permission to implement the deferred temporal model. Branch suggestion: `docs/temporal-model`.

## W13 Historical world state

**Outcome:** an author can ask who owned something or where it was at a chosen fictional time, and see uncertainty or conflicting assertions honestly.

**Slices:** W13a defines the assertion/query model under D10 and the approved W12 time semantics. W13b implements one historical fact family, preferably relationship validity, before location history. W13c tackles age only after birth, elapsed time and calendar interpretation are specified.

**Start in:** canonical relationship and Spatial persistence, temporal queries, capability model and relationship projections. Do not maintain independent duplicated current and historical facts without a specified authority rule.

**Acceptance tests:** historical queries are reproducible; open-ended and overlapping assertions return the approved unknown/conflict results. A historical owner view does not mutate current ownership. Moving a container does not rewrite every descendant's history. Backdated edits trigger explicit review where necessary and retain identity. Future occurrences, Chapter links and Story Roles alone never change world state. Restore, search and projections agree with the same canonical assertions.

**Deferred:** continuity engines, simulation, automatic transitions, character beliefs, route/travel calculations and conflict resolution by silently choosing one assertion.

**Session selection:** W13a after W12's model, implementation only after D10. Branch suggestion: `docs/historical-state-model`.

## W14 Release acceptance and performance

**Outcome:** a named Windows release candidate is trustworthy for everyday writing and large Projects.

**Slices:** W14a establishes a reproducible synthetic integration scenario and records current gaps. W14b verifies each release candidate after its selected workloads land. Fix discovered defects in bounded follow-up PRs; this is not a catch-all refactor.

**Start in:** Milestone 01's canonical integration and torture tests, existing frontend/Rust tests, Windows build procedures from W02 and feature verification notes.

**Acceptance tests:** exercise create, write, connect, navigate, rename, reorganize, Archive/Trash/Restore, backup, close and reopen in the packaged app. Include thousands of Entries and relationships, hundreds of Chapters and occurrences, long names and large supported documents. Measure startup, query latency and editing responsiveness on a documented test machine; propose budgets before treating arbitrary numbers as requirements. Check autosave under rapid switching, process interruption, inaccessible backup storage, display scaling, both themes, keyboard focus and reduced motion. Verify all schema migrations in the candidate's supported upgrade chain and refusal of unsupported newer Projects.

**Deferred:** benchmarking private Projects, public release publication, unrelated features and screen automation without current authorization.

**Session selection:** W14a can start now; a release candidate cannot be called verified based only on component tests or a development browser. Branch suggestion: `test/desktop-release-acceptance`.

## Planning beyond these packets

Maps, draggable graphs, storyboards, multiparty relationships and claims systems each need their own concept review, model, interaction prototype and acceptance packet. Do not quietly add them to W08, W11 or W13. The current list closes foundational gaps and prepares the larger writing and Timeline work without making those later systems prerequisites for a useful desktop app.
