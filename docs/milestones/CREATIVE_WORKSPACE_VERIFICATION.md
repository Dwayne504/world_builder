# Creative workspace — description, reference and idea capture

The author approved this usability slice on 13 October 2026 after describing the application as too much like a glossary. It builds on the consolidated foundations at `047ee7ac26f6722a0f200270f8339adb185a0698`, on `feature/creative-workspace`.

## Author experience

- An Entry opens with its editable Description visible. An empty description is an invitation to write, not a mandatory field or a saved empty document. Existing descriptions and Description-named Fields remain independent.
- Facts, Spatial context and Relationships remain below the writing in a collapsible **Facts & connections** section. Collapsing it does not unmount its editors or discard pending changes.
- Following a connected Entry from an Entry editor opens a saved, read-only reference beside the current page. The same content is used in Chapter link previews; **Keep beside writing** opens the shared reference panel. Descriptions, typed facts, Category/Type, Spatial path and current connections are available without replacing the manuscript editor.
- **Open Entry for editing** uses the existing save/navigation guard. Closing the reference returns focus to its opener. Escape respects an open modal dialog. The reference is a snapshot of committed content; it does not claim to show unsaved edits.
- **Edit → Entry → Capture idea…** accepts a thought and an optional name. It saves a normal Uncategorized Entry and its description together, then opens it. Category and Type can be chosen later. A failed write keeps the creation draft, and dismissing the dialog keeps it available through Add Entry. Existing close guards prevent silently abandoning that draft.

The reference occupies a docked column on a wide window and stacks above the editor on a narrow one. Opening it changes the available width intentionally; hovering does not resize the workspace. The editor stays mounted, preserving selection and undo history.

## Persistence and scope

Idea capture uses the existing serialized Project database worker. Entry identity, metadata, revision and canonical description document commit in one transaction. Empty thoughts and failed document inserts roll back the entire operation. Plain text line breaks become existing supported paragraph nodes. Normal stub Entry creation is unchanged.

There is no new schema or document format, migration, dependency, automatic content generation, or second source of truth. Existing autosave, failed-write recovery, close guards and backup/Restore as Copy remain in use. Supplied real Projects and backups were not opened or changed; all test data was synthetic and disposable.

Deferred: portraits/assets, relationship graph, a new home dashboard, passage links, tabs/restart restoration, AI features, additional Timeline behavior and the other gated roadmap systems. This slice does not complete W14 desktop acceptance.

## Verification

Local checks on Windows:

- `npm run typecheck`, `npm run lint`, `npm run format:check`, `npm run build`: passed.
- `npm test -- --run --maxWorkers=4`: **455 tests passed** across 45 files, without React `act(...)` warnings.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo check`, using the app's `src-tauri` manifest: passed, **319 tests**. Three subprocess helpers are intentionally ignored standalone and exercised by their parent tests.
- Git working-tree and staged whitespace checks: passed.
- GitHub secret scanning and push protection were enabled, with no open alerts when inspected on 13 October. No local secret scanner or CodeQL runner/workflow is configured; no passing run of either is claimed.

Focused coverage includes atomic creation/rollback/retry, reopen and Restore as Copy with Project-scoped identity; failed capture and close protection; a pending description acknowledgement before full navigation; retained editor identity and navigation context; typed reference values including zero/false and Entry References; hidden facts; delayed stale reference responses; retry and keyboard dismissal; and Chapter preview without manuscript replacement or writes. Existing description tests continue to cover corruption, newer documents, failed writes, focus, undo and recovery.

A headless Edge check rendered the real frontend with synthetic in-memory command responses. Both Storybook and Starship themes were inspected, at 1440×1000 and 800×720, with reduced motion. The reference did not cover writing, horizontal overflow was absent, and the manuscript editor remained the same DOM node when the panel opened. This is layout evidence, not native IPC verification.

The native Windows debug build compiled and launched with an isolated app identifier and a synthetic Project. Interactive verification could not proceed: the desktop tool returned a black capture and failed to activate that test window after reconnecting. The test process was stopped. Native WebView2 interaction, actual close-dialog behavior, display scaling and long-running desktop sessions are **not independently verified** here. The existing production JavaScript bundle-size advisory remains.

## Windows manual checklist

1. In a disposable Project, capture an unnamed multiline idea. Confirm it opens once, remains Uncategorized, and survives closing/reopening and Restore as Copy.
2. Type continuously in an Entry description through autosave. Collapse/reopen Facts & connections; confirm writing, field drafts and undo remain intact.
3. Follow a Relationship, Entry Reference Field or Spatial link. Read the reference, continue writing, then close it with Escape and check focus. Open it for editing and use Back to return to the prior context.
4. In a Chapter, expand a linked Entry and choose Keep beside writing. Confirm the manuscript, caret and undo history remain usable, and no content is covered.
5. Repeat with both themes, long names, a narrow window and Windows scaling. Exercise a failed save only on disposable data; verify drafts stay available and navigation does not claim a failed save succeeded.

## Exact source files in this slice

```text
app/src-tauri/src/application/service.rs
app/src-tauri/src/persistence/entry_description.rs
app/src-tauri/src/persistence/worker.rs
app/src-tauri/src/tauri_boundary/commands.rs
app/src/App.test.tsx
app/src/App.tsx
app/src/AppHelp.tsx
app/src/ChapterEditor.test.tsx
app/src/ChapterEditor.tsx
app/src/EntryContextPanel.css
app/src/EntryContextPanel.test.tsx
app/src/EntryContextPanel.tsx
app/src/EntryDescriptionPanel.css
app/src/EntryDescriptionPanel.test.tsx
app/src/EntryDescriptionPanel.tsx
app/src/EntryDocumentPanel.tsx
app/src/RichTextEditor.tsx
app/src/WorkspaceFrame.tsx
app/src/api.ts
app/src/workspaceHistory.ts
docs/milestones/CREATIVE_WORKSPACE_VERIFICATION.md
docs/roadmap/README.md
```

Content-neutral local `Cargo.toml` changes from the desktop build are preserved and excluded from the commit. Temporary fixtures, configuration, screenshots and build output are excluded.
