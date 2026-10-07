# Coding session instructions

Use these instructions with one slice from [the workload packets](WORKLOADS.md). The default outcome is one reviewable PR, with supplied real Projects and backups left untouched.

## Repository preflight

1. Read `.github/copilot-instructions.md`, any applicable `AGENTS.md`, Concept V0.02, Milestone 01, Architecture Proposal V2, and the feature notes relevant to the chosen slice.
2. Inspect status, current branch, recent commits, remotes, local changes and open PRs. Preserve all uncommitted and checkpointed work, including changes that appear to be only line endings.
3. Verify PR #29 is incorporated into the intended base. It was merged into `main` at `7da49ea`; do not silently start from an older local checkout. Use the current remote base, including any later merged dependencies. Never merge a PR yourself.
4. Fetch and use the current remote base. Work on a new appropriately named feature branch or continue an existing unmerged PR for the same slice. Never reset, force-push, discard work or rewrite history.
5. Check the packet's decision gates against recorded author answers. Do not re-request decisions already approved. If a new product question appears, describe it and prepare the reviewable design before implementing behavior that depends on the answer.

Before editing, briefly state the bounded scope, acceptance tests, unresolved decisions and explicit deferrals. A packet's file map is a starting point, not a mandate to keep expanding large components.

## Implementation contract

- Use synthetic Projects in temporary directories. If real data is necessary, copy only the required fixture first; never migrate, rename, delete or write to originals.
- Preserve Project-scoped stable identities. Names are labels. Definitions, authored values and derived search/navigation views remain distinct.
- Reuse the serialized database worker, revision checks, autosave and close guards. A failed or stale write must preserve the draft and must not report success.
- Keep new data in backup and Restore as Copy coverage. Choose the next free migration version at implementation time. Test older supported fixtures, failed migrations and newer-version refusal.
- Keep UI text author-facing. Expose details only where the author needs to make a choice. Reuse the desktop menu, dialogs and existing themes rather than adding permanent buttons everywhere.
- Native desktop behavior is the target. Browser component tests are useful but do not establish Windows WebView2, DPI, file picker, installer or native close behavior.
- Do not include supplied Projects, backups, screenshots containing private material, credentials, personal paths, editor settings or build output in commits.

## Verification contract

For implementation slices, run the following from the repository root and record each result. Run commands separately or check each exit code; a successful final command does not erase an earlier failure.

```powershell
# Frontend, from the app directory
Set-Location app
npm run typecheck
npm test -- --run
npm run lint
npm run format:check
npm run build

# Rust, still from app
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo check --manifest-path src-tauri/Cargo.toml

# Repository
Set-Location ..
git diff --check
git diff --cached --check
```

Run available repository secret scans and CodeQL checks. Inspect configuration first: “not configured” and “not available locally” are distinct from a passing scan. Do not invent a successful security check. CI success also does not prove manual desktop verification.

Add focused regression tests from the chosen packet, not tests that merely mirror implementation. Include keyboard navigation, focus across autosaves, failure states, long names and large synthetic collections where relevant. Check both themes, narrow desktop layouts, reduced motion and no layout shift for changed UI.

For documentation-only design slices, validate facts, links, formatting and the Git diff. A complete frontend and Rust rerun is unnecessary if no executable code or configuration changes. State that limitation clearly.

## Delivery contract

- Review the complete diff and explicitly stage only intended files.
- Commit the completed slice, push if authenticated, and create or update an unmerged PR. Use a human-readable problem statement, scope, tests and limitations. Attach the PR to the coding session where supported.
- Report branch, commit, files changed, check results, unverified behavior and a short Windows manual-test checklist.
- Update the roadmap's completion tracking and record any newly approved product decision. Do not mark an entire workload complete when only one slice is done.
- Leave the next session a specific stopping point. Never merge the PR.

## Reusable session prompt

> Work on Worldcrafter workload **Wxx, slice x** from `docs/roadmap/WORKLOADS.md`. Follow `docs/roadmap/SESSION_START.md` and the relevant decisions in `docs/roadmap/DECISIONS.md`. Verify the current repository and PR state; use `main` only after it contains PR #29 and all required dependencies. State scope, acceptance tests and deferrals before editing. Implement only the selected slice if its behavior is already approved. If an unresolved product decision blocks it, prepare the concrete proposal and ask before implementing dependent behavior. Preserve existing work and all supplied real Projects/backups. Complete the applicable checks, commit and push the result, and open or update an unmerged PR. Do not merge anything.

## First recommended session prompt

> Prepare **W01a**, the automatic backup policy and failure model. Inspect the existing snapshot, recovery, worker and preference code. Produce a short proposal resolving D01, with suggested cadence, retention, snapshot ownership, disk-full behavior, sleep/resume and close behavior. Reuse the existing validated backup format. Show concrete acceptance tests and the implementation breakdown for W01b. Do not enable an unapproved schedule or delete any backups. If D01 has already been approved by the author, record that approval and proceed with W01b under the normal implementation verification contract.

For an implementation session without a pending policy decision, substitute **W03a: rich Entry descriptions** in the reusable prompt.
