# Upcoming development workloads

The next phase should finish the safety and authoring foundations before expanding into advanced world simulation. These workloads turn the remaining Concept V0.02 work into bounded coding sessions, with explicit acceptance tests and decision gates.

Prepared on 7 October 2026 against `main` at `7da49ea`, the merge of PR #29 incorporating `feature/category-settings-trash` and the implementation through PR #28. Future sessions must fetch and verify the current remote state before branching; this commit is a baseline, not a pin to an obsolete checkout.

## Start here

1. Read the [session instructions](SESSION_START.md).
2. Choose one slice from the [workload packets](WORKLOADS.md).
3. Check its [open decisions](DECISIONS.md). Proposed defaults are not approved behavior.
4. Implement and verify that slice, open a PR, and update its status here. Follow the author's current merge authorization in the session instructions.

The ready foundation slices have been consolidated in PRs #31–41 under the author's 12 October merge authorization. PR #38 is the final integration step. Review the [October delivery record](SESSION_2026_10.md), fetch current `main` after that PR lands, then perform W14a native Windows acceptance. Choose only the next necessary product decision from the register before starting a gated workload.

## Recommended order

“Ready” means the documented scope has no known new product decision; it does not waive the session's repository inspection. Each lettered slice is a separate reviewable change, not a promise that an entire workload fits one sitting.

| Workload | Outcome | First slice | Dependencies |
| --- | --- | --- | --- |
| W01 | Rolling automatic backups with safe retention | Merged in #36; native acceptance pending | Existing backup service |
| W02 | Reproducible Windows build and release process | Build proof in #34; distribution still needs D02 | Baseline consolidation; W01 before release readiness |
| W03 | Rich Entry descriptions | Merged in #33; native acceptance pending | Existing Chapter document editor |
| W04 | Complete Type management | Rename #32 and reviewed retirement #40 implemented | Existing Category and Type manager |
| W05 | Rich Text and ordinary Entry Reference Fields | Foundation kinds merged in #39 and #41 | See delivery record for deferred constraints |
| W06 | Tags and named creative Status Systems | Scope decision needed | Stable record identities |
| W07 | Structured Explore and saved views | Core filters implemented in #35; later operators/views remain | W06 for Tag and Status filters |
| W08 | Portable images and attachments | UX and limits decision needed | W01; existing package and document services |
| W09 | Pins, record tabs and restart restoration | Pins/recents implemented in #37; tabs/restart need D06 | Existing navigation and close guards |
| W10 | Manuscript export and writing recovery improvements | W10a Markdown implemented; Word/history remain later | Existing Chapter documents; W01 |
| W11 | Books, Parts and Scenes | Design session first | Existing Chapters; W10 independent |
| W12 | More expressive Timeline dates and calendars | Design session first | Approved Timeline foundation |
| W13 | Historical relationships, locations and age | Design session first | W12 time model |
| W14 | Desktop release acceptance and performance | Ready to establish baseline | Repeat at release candidate after relevant workloads |

This is a priority order, not a requirement to block all authoring work behind release administration. The combined code passed Linux CI and the Windows build workflow: 449 frontend tests, 316 Rust tests and a native executable build. Interactive desktop acceptance remains outstanding; build success does not establish UI, DPI, installer or upgrade behavior.

## What already exists

The current implementation includes Project creation and recovery, directory preferences, manual and operation-specific safety backups, Categories, Types, Entries, unit-bearing Number Fields, relationship projections, Spatial containment, search and aliases, Chapter writing and Story Roles, and a dated Timeline foundation. It also includes desktop menus, recoverable Entry deletion, focused Field management, Chapter browsing and a horizontal Timeline rail. October's consolidated slices add automatic backups, Type renaming and reviewed retirement, Entry descriptions, Rich Text and Entry Reference Fields, Explore, pins and recent records, and Markdown manuscript export.

The remaining Timeline work is about richer dates and historical meaning; an interactive rail is already present. Chapter work should extend the existing library and editor rather than recreate them. Story Roles are functions on explicit Chapter links, not Entry Types or generic tags.

## Scope boundaries

These packets are an implementation plan, not an amendment approving every deferred product system. Approval to prepare workloads does not settle the decisions in the decision register. Keep existing behavior where a question remains open.

Maps and map pins, a draggable relationship graph, storyboards, multiparty relationships, claims and certainty, reusable cross-project libraries, collaboration, cloud sync, mobile clients, plugins and AI assistance remain later discovery topics. No current packet authorizes those features. Decorative effects and additional themes are polish, not prerequisites for the missing foundations.

## Sources and interpretation

- [Concept V0.02](../concept/The_Worldcrafter_Concept_V0.02.docx) defines the product.
- [Milestone 01](../milestones/MILESTONE_01_IMPLEMENTATION.md) defines the core invariants and acceptance scenarios.
- [Architecture Proposal V2](../architecture/MILESTONE_01_ARCHITECTURE_PROPOSAL_V2.md) defines identity, persistence, commands, documents and derived views.
- [Timeline foundation approval](../architecture/TIMELINE_FOUNDATION_PROPOSAL.md) explicitly opens the implemented Timeline slice beyond the older Milestone 01 exclusion.
- [Workspace usability verification](../milestones/WORKSPACE_USABILITY_VERIFICATION.md) records the latest UI work.

Some original milestone checklists and README wording predate implemented features. Use the approved feature notes and current code to determine what is complete. Do not treat an old exclusion as evidence that an implemented, separately approved feature should be removed. Likewise, do not treat a broad long-term concept as a fully specified implementation contract.

## Completion tracking

Use the [October delivery and integration record](SESSION_2026_10.md) for the PRs, fixed implementation commits, exact changed files and combined verification. Earlier feature-branch test counts in individual verification notes are historical, not counts for today's combined build. Do not preallocate future schema versions or recreate a delivered slice.

- W01a–c: [automatic backups](../milestones/AUTOMATIC_BACKUPS_VERIFICATION.md), under approved D01. Native storage and long-running session acceptance remain.
- W09a: [pins and recents](../milestones/WORKSPACE_PINS_VERIFICATION.md). Tabs and automatic restart restoration remain separate.
- W07a: [Explore foundation](../milestones/EXPLORE_FOUNDATION_VERIFICATION.md). Later operators, saved views and Tag/Status filters remain separate.
- W10a: [Markdown export](../milestones/MANUSCRIPT_EXPORT_VERIFICATION.md), under approved D07. Word/EPUB and writing history remain separate.

Next: W14a acceptance of the combined desktop app using disposable Projects. A completed foundation slice does not complete its entire workload.
