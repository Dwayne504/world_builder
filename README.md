## The Worldcrafter application

The Worldcrafter is a local-first desktop application for worldbuilding and story
writing. Product concept and scope are defined in `docs/concept/` and
`docs/milestones/`; approved technical architecture is in `docs/architecture/`.

The application lives in `app/` (Tauri 2 + React 19 + TypeScript frontend, Rust
backend in `app/src-tauri`).

### Prerequisites

* Node.js 20+ and npm
* Rust stable (via `rustup`)
* Linux only: Tauri's native build dependencies, e.g. on Debian/Ubuntu:
  `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev build-essential libssl-dev`

### Install

```sh
cd app
npm install
```

### Development

```sh
cd app
npm run tauri dev   # full desktop app (requires a display/windowing environment)
npm run dev         # frontend only, in a browser, against a mock/no backend
```

### Tests

```sh
cd app
npm test                          # frontend unit tests (vitest)
cd src-tauri && cargo test        # Rust unit/integration tests
```

### Checks / build

```sh
cd app
npm run typecheck                 # TypeScript --noEmit
npm run lint                      # eslint
npm run format:check              # prettier --check
npm run build                     # production frontend build (tsc + vite build)
cd src-tauri
cargo fmt --check
cargo clippy --all-targets
cargo check --bins                # compiles the Tauri binary target
cargo tauri build                 # full desktop bundle; requires a display/windowing
                                   # environment and platform packaging tools, and is
                                   # not expected to succeed in a headless CI/sandbox
```

### Current implemented slice: Project Trust Foundation (Milestone 01, Task 01)

This slice proves the desktop stack starts, a real self-contained `.wcproj`
package can be created/opened/renamed/closed, Project identity (UUIDv7) is
stable and independent of the visible working name, SQLite access is owned by
a dedicated per-Project worker thread (WAL, `synchronous=FULL`, foreign keys
on), renames only report "Saved" after an acknowledged SQLite commit, a second
process cannot open the same Project while a lock is held, and a manual
backup can be created and restored as an independently editable copy with a
new Project ID. See `app/src-tauri/src/` for the module boundaries
(`domain`, `application`, `persistence`, `package`, `backup_recovery`,
`tauri_boundary`) and their automated tests.

**Current non-goals** (deliberately out of scope for this slice): Categories,
Types, Entries, Fields, Relationships, Spatial, Chapters/Story Units, TipTap
prose editing, search/FTS, Tags/Roles/Statuses, Archive/Trash UI, final
Recent/Pinned navigation, and any final visual design system.

### Manual native-close verification

In a packaged Tauri build, verify title-bar and OS close shortcuts: clean Projects
close their backend session before the window exits; dirty, saving, and failed
renames keep the window open until an explicit discard; backend-close failures
remain visible and do not exit; and the in-app **Close Project** button returns
to Home without exiting the application.

### Engineering spike notes affecting later slices

* **Project locking is age-based, not PID-liveness-based.** Cross-platform
  liveness checks for another process's PID are unreliable (especially over
  network/cloud-synced filesystems), so a lock is only considered "stale"
  after a fixed inactivity threshold; recovering a stale lock is always an
  explicit, separate operation and is never automatic.
* **`ProjectDbWorker` is a single dedicated OS thread per open Project**
  owning the one `rusqlite::Connection`, driven by a serialized job queue
  over `std::sync::mpsc` channels (no async pseudocode wrapping a shared
  connection). Spawning synchronously validates identity, pragmas, and
  migrations before the open call returns, so open failures are reported
  rather than surfacing later.
* **Backups are consistent snapshot directories, not compressed archives.**
  This slice uses SQLite's Online Backup API against the live worker
  connection (safe to run while WAL is active) and copies the manifest and
  managed package directories alongside it, but does not compress the result
  into a `.wcbackup` archive; a compression step can be layered on later
  without changing the snapshot/validation contract.
* **No file-picker dialogs are used yet.** The minimal UI takes filesystem
  paths as plain text input to avoid adding a dialog plugin dependency before
  it is genuinely needed.
