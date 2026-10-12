# Markdown manuscript export (W10a)

Implemented against the merged roadmap baseline `e8267f9` under D07, approved by
the author on 7 October 2026. Synthetic temporary Projects and destinations were
used throughout. Supplied Projects and backups were not opened or modified.

## Author workflow

In the desktop app, choose **File → Export manuscript…**. Search and select
Chapters in a paged list, then prepare a preview. Selection starts empty and the
list starts with active Chapters; archived or trashed Chapters require explicit
selection and their state appears in the reviewed scope. The output follows
reading order, independently of selection order.

Preview first drains pending writing through existing save controllers. Failed
saves or unfinished structural forms block it while retaining the draft. The
preview lists the chosen Chapters, word counts and exact inert Markdown text.
Native Save As chooses the destination; replacing a file requires a separate
explicit Replace action. Cancellation is quiet and writes no file. A foreground
export operation finishes before a requested native Project/window close.

Only manuscripts are exported, with Chapter-title headings and UTF-8 text.
Plan and Notes are excluded. This baseline has no separate Chapter Summary
column/document; the explicit manuscript-only query also excludes any future
Summary area. Export is a derived file, not a Project backup, and never changes
canonical prose, Story links, reading order or Project revisions.

Supported writing marks and styled numbering are preserved using generated safe
HTML where necessary inside Markdown. The preview explains that some readers
hide underline or styled numbering. Literal author text resembling HTML, links or
Markdown syntax stays text. Word, EPUB, print layout and writing revision history
remain separate slices.

## Publication and recovery

Previews and reviewed destinations use bounded, expiring native tokens tied to an
open Project session. Before publication the worker rereads the selected writing
and rejects changed Project revisions or changed canonical content. The reviewed
destination is checked by file identity, size, modification time and content hash.
Replacing it with a different file, even with the same bytes/time, requires review.

The complete output is staged and flushed in the destination directory. Windows
publication uses a native same-volume move without replacement; an unexpected
destination fails rather than being overwritten. A reviewed old file is preserved
through a small owned recovery protocol until publication succeeds. Failures
restore it where possible, or identify the exact preserved recovery file when a
concurrent destination prevents restoration. Save As never silently restores an
old recovery file. Interrupted evidence remains available for inspection.

Project/backup packages, linked or redirected paths, Windows device names and
alternate data streams are rejected. Known non-redirecting Windows Cloud Files
tags are permitted. The non-Windows implementation uses a no-clobber hard-link
publication fallback; unsupported filesystem capabilities fail visibly. This is
not a claim that physical FAT/exFAT media, network shares or OneDrive hydration
were independently tested.

## Independent reader verification

The actual synthetic file produced by the export integration test was opened and
rendered with **markdown-it-py 4.0.0**, using CommonMark with HTML enabled. The
rendered document was independently inspected with Python's HTML parser. Checks
passed for Chapter order, Unicode, all five inline marks and adjoining mixed
marks, headings 1–3, hard breaks, rules, quotes, nested bullet/numbered lists and
Roman numbering. Literal HTML/link syntax remained text, leading indentation did
not become a code block, and no unintended script, link or image elements appeared.
Plan/Notes sentinel text was absent. The source Chapter snapshot stayed unchanged.
The reader and its artifacts were temporary test tools, not app dependencies.

## Automated checks

- Frontend: typecheck, all 347 tests (33 files, one worker), lint, formatting and
  production build passed. The existing bundle-size advisory remains.
- Rust: all 246 tests passed; three subprocess helper tests are ignored directly
  and exercised by their parent tests. Formatting, strict all-target Clippy and
  `cargo check` passed. This includes native Windows no-clobber publication,
  open-writer denial, junction refusal, changed-file identity and injected
  publication/recovery failures.
- Repository diff checks passed. No local CodeQL or secret-scanning command is
  configured. GitHub security checks are recorded separately from test results.
- This branch retains the baseline test-tool dependency advisories; the separate
  `fix/test-tooling-security` PR resolves them. It is not silently folded into
  this export slice.

## Windows manual acceptance

Use a disposable Project and export directory.

1. Write formatted Unicode text in several Chapter manuscripts, with different
   Plan/Notes content. Select Chapters out of order and verify the preview and
   exported file follow reading order and include only manuscripts.
2. Check the picker, paging, long names, unnamed Chapters and explicit inactive
   selections with keyboard and pointer, both themes and Windows scaling.
3. Cancel Save As. Export a new file, then explicitly replace a disposable file.
   Open the result in the intended Markdown reader and inspect its formatting.
4. Change the chosen file externally between review and Export. Confirm refusal
   without loss of either version. Test an unavailable destination and retry.
5. Export while writing is pending, and request window close during export. Confirm
   writing finishes first and a failed save keeps the draft open for review.

Native picker interaction, visual/DPI acceptance, physical power loss, external
drive filesystems and network/cloud storage remain unverified. Backend Windows
filesystem tests and component tests do not establish all those scenarios.
