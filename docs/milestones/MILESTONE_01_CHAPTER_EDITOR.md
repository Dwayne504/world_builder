# Milestone 01 — Chapter editor

This slice implements the loose Chapter workflow from Concept V0.02, Milestone 01
sections 19–23, and approved Architecture Proposal V2 sections 9–10. Chapters are
Story Units, independent from world Entries. A Project can start with a Chapter
and no world material. No Book, Category, title, or Story Role is required.

## Author workflow

- **Chapters** in the desktop sidebar opens the reading-order list. Create an
  untitled Chapter, rename its heading, and move it up or down. Reading order is
  not fictional chronology.
- **Manuscript**, **Plan**, and **Notes** are three separate rich-text documents.
  The manuscript is primary and its word count excludes planning and notes.
  The toolbar supports bold, italic, headings, lists, quotes, and undo/redo.
- The collapsible right context panel lists linked Entries. Expand an Entry to
  preview Fields, current Relationships, and its separately labelled derived
  Spatial path. Open the full Entry and return using workspace history.
- Search for an Entry to link it; the picker shows at most ten matching results.
  Links may have no Roles, or several. The Roles button beside each linked Entry
  opens its assignments directly. Chapter options lists available Project Roles,
  their assignments in this Chapter, and Find uses across the Project. Creating
  a Role makes it available without assigning it. Role assignments never duplicate
  the underlying link. See [Role discovery](MILESTONE_01_STORY_ROLE_DISCOVERY.md).
- Entries show **Story usage**, derived from those same Chapter links. Renaming
  an Entry updates the resolved link label without rewriting authored prose.
- Chapter options offer Archive and Trash. Both retain documents and links;
  Chapters can be read and restored from their corresponding library view.
  Permanent Chapter deletion is not exposed by this slice.

## Persistence and recovery

Schema 9 adds Story Units, owned rich documents, Role definitions, canonical
Chapter–Entry links, and link–Role assignments. Stable UUID identities and
restrictive foreign keys keep these separate from Entries. The existing package
migration mechanism takes an external recovery snapshot before upgrading an
older Project. Failed migrations roll back and can be retried. Restore as Copy
changes only Project identity while preserving snapshot-internal Chapter,
document, Entry, link, and Role IDs.

Every Story mutation goes through the existing serialized database worker and
an immediate SQLite transaction with the expected Project revision. Document
text, Unicode word count, metadata, and revision commit together. Reordering is
atomic; it does not rewrite prose or links. Archive/Trash has one authority in
the identity registry.

Document schema 1 is a bounded TipTap/ProseMirror AST. Rust validates the version,
node hierarchy, formatting marks, attributes, size, and depth before a write.
Unknown, corrupt, or newer documents retain their original JSON and available
plain text in a read-only recovery view. Their other writing areas remain
editable. Images, raw HTML, scripts, and embedded remote content are not part of
the supported document format.

Typing uses a short debounce; blur, navigation, and native close flush pending
writing, including text entered while an earlier save was awaiting acknowledgement.
Saved means the database worker acknowledged the durable commit. A save failure
keeps the draft and warns visibly; typing does not automatically retry or rebase
against a newer revision. **Review saved version** offers an explicit comparison
before restoring a conflicting draft. Formatting acknowledgements do not reset
the editor, cursor, or undo history.

A best-effort WebView-local emergency draft lives outside the Project, keyed by
Project and Chapter ID. It is not a backup or a Saved acknowledgement. On recovery,
the author compares saved and recovered writing before choosing which to use.
Malformed, unsupported, or lossy-to-render recovery data is preserved for copying.
Storage failure is visible; the in-memory draft remains available. This buffer
does not guarantee recovery after storage failure, disk failure, or power loss.

## Validation

Automated Rust coverage includes separate documents and identities, role-free and
multi-Role links, backlinks, rename preservation, reorder rollback, stale and
concurrent writes, corrupt/newer documents, AST validation, process exit after an
acknowledged save, unfinished-transaction rollback, schema-8 upgrade rollback and
recovery snapshot, and backup/Restore-as-Copy integrity. All fixtures are synthetic
temporary Projects. No supplied author Project or backup is opened for writing.

Frontend coverage includes delayed acknowledgements, continued typing, native
close flushing, navigation failure, recovery review, unsupported documents,
local-storage failure, library actions, and bounded Entry search. Existing
navigation tests also cover focus restoration after asynchronous linked content
loads. Headless Edge checks exercise both themes, typing/autosave focus,
formatting/undo, separate Plan/Notes, context toggling, Entry round trips, and
overflow at desktop and narrow widths with an in-memory API fixture.

Required gates: frontend typecheck, tests, lint, formatting and production build;
Rust formatting, tests, Clippy with warnings denied and compile check; repository
whitespace and forbidden-artifact review. Headless Edge uses mocked IPC and does
not replace a native WebView2/manual test. Native close is covered by the frontend
close-event regression and real Rust persistence tests, not a manual OS-close run.

Local verification on 2026-09-30 passed: 214 frontend tests; 194 Rust tests
(two subprocess helpers are marked ignored for normal discovery and invoked by
their parent crash tests); typecheck; ESLint; Prettier; production build; Cargo
format, Clippy and check; and Git whitespace checks. The build reports a
non-blocking JavaScript chunk-size warning after adding the rich-text engine.
The production dependency audit reports no vulnerabilities. Two moderate
development-tool advisories remain in the existing Vitest toolchain; fixing
them requires a separate major-version test-tool upgrade.

GitHub secret scanning and push protection are enabled, with zero open secret
alerts at verification. No repository secret-scanner CLI/script is configured;
a supplemental diff review checks credential patterns and forbidden artifacts.
CodeQL default setup is `not-configured`, with no workflow or local CLI, so no
CodeQL pass is claimed.

## Windows manual checklist

Use a disposable Project, or a copy of an existing Project.

1. Open **Chapters**, create two Chapters, rename and reorder them, then reopen
   the Project. Verify text, order, formatting, and manuscript-only word count.
2. Type in all three writing areas. Keep typing across autosave; check caret and
   undo. Type a final sentence and immediately close using the window title bar;
   reopen and verify that sentence.
3. Link a person, a Spatial place, and an object. Assign two Roles to the person
   and no Role to the object. Check previews, derived place ancestry, and Story
   usage. Rename the person: the link changes but the manuscript does not.
4. Open an Entry from the context panel, go Back, and continue writing. Toggle
   the context and sidebar and check both appearance choices and window sizes.
5. Archive, restore, Trash, and restore a Chapter. Verify its writing and links.
   Back up the Project and restore it as a copy; edit the copy independently.

## Explicit deferrals

Books/Parts/Scenes, timelines and calendars, passage-level links and quick-create
from selected prose, graph views, multi-pane split editing, publishing/export,
rich Entry-description Fields, Tags/Statuses, permanent
Chapter deletion, and Role rename/retirement management remain separate work.
Global Story search/FTS was added by the subsequent Search slice.
This slice makes no claim that all of Milestone 01 is finished.
