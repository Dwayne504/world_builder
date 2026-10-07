# Upcoming development workloads

The next phase should finish the safety and authoring foundations before expanding into advanced world simulation. These workloads turn the remaining Concept V0.02 work into bounded coding sessions, with explicit acceptance tests and decision gates.

Prepared on 7 October 2026 against `main` at `7da49ea`, the merge of PR #29 incorporating `feature/category-settings-trash` and the implementation through PR #28. Future sessions must fetch and verify the current remote state before branching; this commit is a baseline, not a pin to an obsolete checkout.

## Start here

1. Read the [session instructions](SESSION_START.md).
2. Choose one slice from the [workload packets](WORKLOADS.md).
3. Check its [open decisions](DECISIONS.md). Proposed defaults are not approved behavior.
4. Implement and verify that slice, open an unmerged PR, and update its status here.

The first recommended session is **W01a: automatic backup policy and failure model**. Once that policy is approved, implement W01b. If the author is unavailable for a decision, **W03a: rich Entry descriptions** is the first substantial implementation slice that can proceed under the existing design.

## Recommended order

“Ready” means the documented scope has no known new product decision; it does not waive the session's repository inspection. Each lettered slice is a separate reviewable change, not a promise that an entire workload fits one sitting.

| Workload | Outcome | First slice | Dependencies |
| --- | --- | --- | --- |
| W01 | Rolling automatic backups with safe retention | Decision needed | Existing backup service |
| W02 | Reproducible Windows build and release process | Ready for build verification | Baseline consolidation; W01 before release readiness |
| W03 | Rich Entry descriptions | Implemented; review and native acceptance pending | [Verification](../milestones/ENTRY_DESCRIPTIONS_VERIFICATION.md) |
| W04 | Complete Type management | Rename ready; lifecycle decision needed | Existing Category and Type manager |
| W05 | Rich Text and ordinary Entry Reference Fields | Ready within existing Field semantics | W03 shared document work recommended |
| W06 | Tags and named creative Status Systems | Scope decision needed | Stable record identities |
| W07 | Structured Explore and saved views | Core filters ready | W06 only for Tag and Status filters |
| W08 | Portable images and attachments | UX and limits decision needed | W01; existing package and document services |
| W09 | Pins, record tabs and restart restoration | W09a implemented; tabs/restart still need D06 | Existing navigation and close guards |
| W10 | Manuscript export and writing recovery improvements | Export decision needed | Existing Chapter documents; W01 |
| W11 | Books, Parts and Scenes | Design session first | Existing Chapters; W10 independent |
| W12 | More expressive Timeline dates and calendars | Design session first | Approved Timeline foundation |
| W13 | Historical relationships, locations and age | Design session first | W12 time model |
| W14 | Desktop release acceptance and performance | Ready to establish baseline | Repeat at release candidate after relevant workloads |

This is a priority order, not a requirement to block all authoring work behind release administration. W02 build verification, W03, W04a and W07a can proceed independently of the unresolved backup policy. Do not combine them into one large PR.

## What already exists

The current implementation includes Project creation and recovery, directory preferences, manual and operation-specific safety backups, Categories, Types, Entries, unit-bearing Number Fields, relationship projections, Spatial containment, search and aliases, Chapter writing and Story Roles, and a dated Timeline foundation. It also includes desktop menus, recoverable Entry deletion, focused Field management, Chapter browsing and a horizontal Timeline rail.

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

W03 is implemented on `feature/entry-descriptions`: optional rich descriptions, revision-safe autosave/recovery, migration and Search, with 353 frontend and 244 Rust tests passing. See its verification note for native acceptance still outstanding. Other workloads remain planned unless their own delivered slice records a later status. Do not guess future merge SHAs or preallocate future database schema versions.

W09a is implemented on `feature/workspace-pins`, stacked on W03. See [pins and recents verification](../milestones/WORKSPACE_PINS_VERIFICATION.md). This completes the shortcut slice, not tabs or automatic restart restoration.
