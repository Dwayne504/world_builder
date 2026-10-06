# Workspace usability pass

This presentation-only follow-up builds on the desktop menus in PR #27. It does
not change Project storage, schemas, calendar arithmetic, relationship identity,
or the meaning of deletion and defaults. Real Projects and backups were not used
or modified. All large-library tests use synthetic data.

## Navigation and browsing

- Entries: search by name, 20/50/100 rows per page, Category groups and exact Type
  filtering. Back restores the Entry search and page. Sidebar counts remain the
  full active Category totals.
- Chapters: 20 compact rows, title/reading-position search, display sorting,
  word counts, remembered session browsing, and direct movement to a reading position. Filtering and display
  sorting do not reorder the manuscript; the explicit Move dialog uses the full
  Writing order, including Chapters hidden by search.
- Chapter editing: a collapsible context panel, searchable paged Entry links and
  Story Roles, and grouped lifecycle options. Manuscript, Plan and Notes editors
  remain mounted; hiding a panel does not discard writing or unfinished forms.
- Relationships: direct text search and collapsible detailed filters. Grouped
  relationships preview three participants, with search and bounded expansion.
  Restored relationship targets remain available even outside the initial batch.
- Categories, Types, Fields, projections and merge/reassignment dialogs: searchable
  native selectors keep the selected ID visible, with manageable batches and
  secondary maintenance actions behind disclosures. No shared-data operation
  changes meaning or bypasses its existing review.
- Entry settings: separate Category & Type, Other names, and Archive & delete
  sections. Drafts remain mounted and navigation/close guards still apply.
- All dialogs: a compact fixed header keeps Done reachable while the body scrolls.
  Long titles remain accessible in full but are visually bounded.

Search, Spatial structure, backups, preferences, appearance and creation windows
retain their existing workflows and receive the shared spacing/dialog treatment;
Spatial creation also uses a searchable Category picker. The existing top menu
continues to expose context-specific management actions.

## Timeline and pointer feedback

Timeline has an optional horizontal chronology and a compact list alternative.
Dates are grouped using the existing fictional calendar; same-day moments share
one date label, and undated ideas stay separate. **Event spacing is not to scale**:
the rail does not invent durations, times of day, or an order within a shared date.
It starts with ten moments, expands to at most fifty per page, and supports search,
linked Entry/Chapter filters, horizontal scrolling, arrow-key navigation and touch.
Changing page or filters resets rail scroll; saving and showing more preserve it.
Large occurrence link selections and navigation are searched and paged too.

Sidebar labels and timeline dots respond to nearby mouse movement using transforms
inside fixed hit areas. Sidebar width, row sizes and the content column stay fixed.
Long Category names have full-name tooltips. Pointer light reaches surfaces as
well as the decorative background; decorations cannot receive clicks or focus.
Reduced-motion and forced-colour preferences disable the decorative effects.

## Verification

Completed on 2026-10-06:

- `npm run typecheck`
- `npm test -- --run`: **332 tests passed in 32 files**, no React act warnings
- `npm run lint`
- `npm run format:check`
- `npm run build` (existing large-bundle advisory remains)
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo test --manifest-path src-tauri/Cargo.toml`: **232 passed**; three
  subprocess helpers are ignored individually and exercised by their parent tests
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `git diff --check`

Focused tests cover 100+ record collections, 1,000 occurrence links, restored
selected IDs, canonical Chapter moves while filtered/sorted, paged navigation,
hidden drafts, keyboard focus, hover cleanup and reduced-motion changes. Windows
sandbox dependency resolution raised EPERM during an initial Vitest attempt;
the complete authorized unsandboxed run passed.

Headless Edge exercised 18 pages/dialogs in Storybook and Starship at 640×600,
1180×960 and 1600×960, using 125 Entries, 120 Chapters, 125 moments, and more than
100 definitions/Types/Roles. It checked page overflow, dialog header reachability,
Entry settings draft retention, keyboard timeline access, bounded lists, pointer
light activation, reduced motion and unchanged sidebar/content bounds. Selected
screenshots were inspected for contrast, density and narrow-window layout.

These browser checks use synthetic IPC responses. They supplement the real Rust
persistence/locking tests and do not independently verify Windows WebView2 rendering,
native window closure, multi-monitor DPI, or a packaged installer.

GitHub secret scanning and push protection are enabled; no open secret alerts were
reported. This repository has no CodeQL workflow or local CodeQL/secret-scanner
executable. No CodeQL run is claimed.

## Windows manual checklist

Use a disposable Project or a copy of a real Project, in both visual themes.

1. Browse a large Chapter library; search, change display order, move a Chapter,
   open it, and return. Verify the reading order changes only through Move.
2. Edit manuscript text and an unfinished Role/alias/default form. Hide/reopen
   panels, navigate and close the app; verify drafts and failure guards remain.
3. Open every Edit-menu dialog. Search long Type/Field/Entry lists, confirm current
   selections stay selected, and reach Done without scrolling the whole window.
4. Explore Timeline with mouse, scrollbar and keyboard. Check same-day/undated
   moments, filters, next page, notes, and links. Compare with List view.
5. Hover near sidebar labels and timeline dots. Confirm the editor never shifts,
   input remains clickable, and labels are readable at 100%, 150% and 200% DPI.
6. Enable Windows reduced motion/high contrast. Confirm the interface remains
   usable without illumination or magnification.
