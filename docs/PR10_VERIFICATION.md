# PR #10 recovery and verification

## Handoff audit

The existing `project-location-preferences` branch was clean at checkpoint
`a19b4e9b40a8d34bd487cedac1d9f33da4a1411a`. PR #10 was open and unmerged at
`94024097496ed04439965e9737ce6d246c5f7877`; remote main was
`9aad758ece59aa9aff9467f6048c9907c44d882a`. The checkpoint and its parent changes
were retained. No history was rewritten.

Reviewed `.github/copilot-instructions.md`, the root and app READMEs, Concept
V0.02, Milestone 01, Architecture Proposal V2 (explicitly approved), and CI.
This change remains within PR #10 and implements no new milestone features.

Read-only corpus inspection found four projects and three directory backups,
all package format 1, with schema versions 1 and 2. Layouts include manifests,
SQLite databases, managed directories, and three projects with leftover lock
metadata. No manifest publication sidecars were present. No database was opened,
no original was written, and no private contents, project IDs, or paths are
included here. Automated tests use synthetic temporary fixtures.

| Requirement | Handoff finding | Completed behavior / verification |
| --- | --- | --- |
| App Projects/Backups defaults | Implemented; asynchronous defaults could overwrite manual choices | Defaults apply in the same session; delayed loads preserve manual paths; clearing a default clears an untouched creation location |
| Native folder selection | Implemented; blocking command and unhandled UI failures | Picker runs on a blocking worker; cancellation preserves input; errors are visible |
| Windows package names and backend preview | Implemented, but suffixing `CON.txt` remained reserved | Prefix reserved base names, including superscript device names; bound UTF-8 component bytes; Rust supplies preview and creation names |
| Backup organization | Already grouped by Project ID | Explicit path assertion added to backup/restore test |
| Immediate lock recovery | Complete in existing PR | Fresh orphan recovery and active-owner refusal pass; no age gate remains |
| VS Code command auto-approval | Removed by checkpoint | Root settings absent; `.vscode/` ignored; no machine settings added |
| Preference publication/recovery | Protocol present; errors and incompatible sidecars could become defaults | Recovery errors stay visible; newer candidates fail closed; successor-write failure preserves prior publication |
| Schema validation | Parsed complete old shape before version | Version checked first; changed newer shapes refuse load, update, and reset |
| Corrupt preferences | Visible reset existed; sidecar evidence incomplete | Reset preserves current and recovery sidecars under unique diagnostic names; unsupported schemas have no reset action |
| Serialized read-modify-write | One store mutex only | Mutex plus shared OS file lock; independent stores retain concurrent updates |
| Directory validation | Only `is_dir` | Absolute, accessible existing directory outside `.wcproj`/`.wcbackup`, including canonical ancestors; operation destinations rechecked |
| Formatting and React warnings | Frontend baseline passed without warnings | Full checks pass without `act(...)` warnings; modified files formatted |
| Existing usability cleanup | Present | Retained labeled inline forms, cancel actions, focus styles and location/error display |

## Checks

Run from `app/`:

- `npm run typecheck`
- `npm test -- --run` — 71 tests in 4 files
- `npm run lint`
- `npm run format:check`
- `npm run build`

Run from `app/src-tauri/`:

- `cargo fmt --check`
- `cargo test` — 93 unit tests and 11 integration tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`

Repository: `git diff --check`. A supplemental credential-pattern scan of
changed tracked files found no matches. No repository secret-scan or CodeQL
workflow/script, or installed scanner executable, was available. The existing
GitHub check configuration exposed only the `verify` CI job. The supplemental
scan is not a substitute for a dedicated secret scanner or CodeQL analysis.

Initial sandbox attempts blocked esbuild configuration resolution and Rust
temporary-path checks. Reruns outside the sandbox passed. Build products remain
ignored and are not committed.

## Verification limits and Windows manual checklist

Automated filesystem tests ran on Windows. Native dialogs and a packaged GUI,
actual process-kill/power-loss behavior, network shares, and cloud-sync races
were not exercised. Recovery tests construct interrupted-publication states;
directory sync is best-effort where the OS does not support it. Directory
validation does not guarantee future write access or free space; operation
failures remain visible.

Use disposable copies and temporary locations for these checks:

1. Choose Projects and Backups defaults; cancel/reopen dialogs; restart the app
   and confirm persistence. Verify manual overrides survive default loading.
2. Create names such as `CON.txt`, `LPT¹`, trailing dots, invalid punctuation,
   and a long Unicode name. Confirm preview equals the created location and
   collisions leave existing packages unchanged.
3. Create a backup, verify its Project-ID folder, and restore as an independent
   copy. Changing defaults must not relocate existing data.
4. Open a disposable project in one instance; a second must refuse takeover.
   Kill the first, reopen immediately, and explicitly recover without waiting.
5. With the app closed, corrupt a disposable preferences file. Confirm visible
   warning, manual operations, explicit reset, and preserved diagnostic bytes.
   A newer schema must refuse updates and reset.
6. Exercise missing folders, file paths, relative paths, package-internal paths,
   and denied write access. Verify visible failures and no fallback location.
7. Run two instances and change different defaults; restart and verify both
   survive. Verify title-bar close and unsaved-change handling still work.

## Files changed after the checkpoint

- `README.md`
- `docs/PR10_VERIFICATION.md`
- `app/src/App.tsx`
- `app/src/App.test.tsx`
- `app/src/api.ts`
- `app/src-tauri/src/atomic_file.rs`
- `app/src-tauri/src/backup_recovery/mod.rs`
- `app/src-tauri/src/package/layout.rs`
- `app/src-tauri/src/preferences/error.rs`
- `app/src-tauri/src/preferences/mod.rs`
- `app/src-tauri/src/preferences/store.rs`
- `app/src-tauri/src/tauri_boundary/commands.rs`
