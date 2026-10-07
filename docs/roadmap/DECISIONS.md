# Product decisions for upcoming work

These are open choices, not approved defaults. Resolve only the decisions needed for the selected workload. Record the author's answer, date and affected packet here; avoid asking for the entire roadmap at once.

## D01 Automatic backup policy

Required before W01b. The concept requires rolling automatic backups but does not settle a concrete cadence or retention count for the current implementation.

Proposal for review: back up changed Projects after a configurable interval while open, coalesce missed intervals after sleep, and bound retained automatic snapshots. Preserve manual and operation-specific safety snapshots unless the author explicitly removes them. Decide the initial interval/count, whether automatic backups start enabled, whether to offer a final backup on close, and how storage exhaustion is presented. Pruning may touch only positively identified automatic snapshots for that Project, after a new snapshot validates successfully. A folder name or file age alone must never confer permission to delete.

## D02 Windows release distribution

W02a may establish a reproducible local installer build without this decision. Before publication, decide the intended installer format, release version/channel, signing arrangement and whether to publish an unsigned test build. Recommendation: a manually installed Windows test release first; automatic updating is a separate approved slice with signing, rollback and schema compatibility considered together. Do not add an updater or publish a release merely because the build succeeds.

## D03 Type lifecycle

W04a renaming is already supported by the identity rules. Before retirement/removal, settle the visible behavior for existing Entries, child Types, inherited defaults and restoration. Recommendation: retirement prevents new assignment while preserving existing assignments and authored values, with a reviewed impact list. Permanent definition deletion and automatic child reparenting remain outside that first slice. Confirm how a retired parent contributes defaults before implementation; do not silently drop inherited structure.

## D04 Tag and Status scope

Before W06, decide the initial assignable record kinds. Recommendation: start with Entries and Chapters, leaving occurrences and relationship instances for explicit follow-up. Confirm whether to seed any named Status Systems or start empty. Tags remain loose labels; Status Systems each allow their own current value. Canon, writing progress, Archive/Trash and current/past relationship state must not become one mixed status menu.

## D05 Managed assets

Before W08, confirm first supported media, size limits, attachment placement and removal behavior. Recommendation: locally imported images and generic downloadable attachments stored inside the package, with no remote embeds or externally linked files. Removal should first detach the reference; asset cleanup needs a separate review of shared references, Trash and backup coverage. Do not silently delete the source file or all shared copies.

## D06 Tabs and restart behavior

W09a pins can use explicit author-controlled ordering. Before tabs and restart restoration, settle whether opening a record replaces a preview tab or creates a persistent tab, how duplicate opens behave, and whether reopening the last Project is opt-in. Recommendation: explicit “Open in new tab,” bounded recents and restoration only after the Project is safely opened. Session metadata is not an authoritative copy of content and must never bypass lock or newer-schema checks.

## D07 Export and writing history

Approved by the author on 7 October 2026: first export is UTF-8 Markdown of selected Chapters' manuscripts in reading order, excluding Plan, Notes and Summary. Show the chosen scope before export. Word export follows later; EPUB remains deferred. Export does not rewrite manuscript content or serve as a Project backup.

Writing revision history still needs its own retention, restore and conflict policy; existing emergency draft recovery must not be presented as a full version-history system.

## D08 Story hierarchy

Before W11 implementation, choose the first hierarchy and placement rules: Book/Part/Chapter, or Chapter/Scene, and whether a unit has one parent. Decide how loose Chapters, manual reading order, moves, archive/trash and export traversal behave. Recommendation: preserve loose Chapters and introduce one hierarchy increment at a time. Reusing the same Chapter in multiple Books and alternate story variants are separate decisions, not implied by adding folders.

## D09 Temporal precision and calendars

Before W12 implementation, define exact versus partial versus approximate dates, uncertain bounds, ranges, open ends, duration semantics and ordering when dates overlap. Then separately define time of day, leap rules, epochs, multiple calendars and conversion. Recommendation: precision and ranges first, keeping the existing one-calendar model until conversion rules are approved. Approximate dates must not be silently converted to exact coordinates for historical queries. Review ambiguous examples before choosing a canonical representation.

## D10 Historical world state

Before W13 implementation, decide how dated relationship and location assertions coexist with today's current-state commands, how conflicting assertions are shown, and what an unknown answer means. Decide whether age comes from an explicit birth occurrence and which calendar measures it. Recommendation: begin with explicit authored assertions and read-only “at this time” views. Do not infer state changes from prose, Story Roles or the mere presence of an occurrence, and do not silently rewrite current relationships when a date passes.

## Approval record

No new decisions in this register were approved by preparing this roadmap. Previously implemented approvals, including custom Number units, reviewed Field merging and the Timeline foundation, remain in force. Future answers should be recorded under the corresponding decision without rewriting that history.
