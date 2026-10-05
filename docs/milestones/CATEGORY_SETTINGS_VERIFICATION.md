# Category settings and recoverable Entry deletion

This slice adds Category settings to each Category's browsing page and exposes the existing Entry lifecycle through Archive and Trash. It follows Concept V0.02 and Milestone 01: Category deletion reassigns Entries; authored records and links are not cascaded away.

## Behavior

- Category settings opens the selected Category, with its existing Types, Field defaults and Spatial defaults. Rename keeps the Category's identity.
- Delete Category requires a destination Category, including Uncategorized if desired. The review includes Entries in Archive and Trash. If the Category owns Types, the author must explicitly confirm their removal and clearing those assignments. Shared Field definitions, filled-in values, owned capabilities, Spatial containment and semantic links survive.
- A recovery backup must succeed before Category deletion. It is recorded under Backups. Restore as Copy can recover the previous Category and Types without overwriting the current Project.
- Uncategorized remains the protected fallback and cannot be renamed or deleted.
- Entry settings offers Archive and Delete Entry. Delete asks for confirmation and moves only that Entry to Trash. Both states preserve its material, Relationships, Chapter/Timeline links and Spatial children.
- The Entries page's View selector opens Active Entries, Archive or Trash. Open an inactive Entry and use Restore Entry to make it active and editable again. Existing links still resolve by identity.
- Normal Entry selectors and sidebar counts use active Entries. Search's existing include-inactive behavior remains available.
- Pending Entry edits are flushed before Archive/Trash; failed saves keep the editor open. Pending mutations participate in the existing close/navigation guards. Hidden Category rename drafts retain their intent, including an empty name draft.
- Era label help explains that it is a display suffix after the fictional year, without changing dates or arithmetic. Event page help distinguishes the Entry describing the event itself from contextual people/places linked to the occurrence.

Permanent Entry deletion, bulk operations, and changes to Timeline/calendar semantics are deferred.

## Persistence and tests

No schema migration or dependency change. Lifecycle state remains in `record_identity`; the Entry DTO now exposes it. Category deletion, reassignment, default detachment and revision advancement occur in one transaction. The backend rechecks the reviewed global revision, destination and Type confirmation under the Project worker lock, including while making the recovery backup.

Focused Rust tests use disposable synthetic Projects and cover populated Category deletion across active/archived/trashed Entries, Type inheritance, retained Field values and Spatial state, successful backup/Restore as Copy, reopening, invalid and stale commands, failed backup, injected transaction failure, and reversible Trash with Story, Timeline and Relationship references.

Focused UI tests cover the selected Category settings, empty/dismissed rename drafts, protected Uncategorized, reviewed Category deletion, backup failure and pending acknowledgements, Entry delete confirmation/cancel, flushing the latest title, failure retention, and Trash restoration. Existing native-close and editor regression tests remain part of the complete suite.

Verification on Windows: all checks below passed, with 254 frontend tests and 228 Rust tests. Three ignored Rust helpers are invoked by their subprocess integration tests. One earlier frontend run timed out in an existing preferences test; the complete rerun passed without extending timeouts.

- `npm run typecheck`
- `npm test -- --run`
- `npm run lint`
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

The production build retains the existing bundle-size advisory. GitHub secret scanning and push protection are enabled; the repository reported zero open secret alerts during this slice. There is no repository secret-scan script, local Gitleaks/CodeQL CLI, or CodeQL workflow; the code-scanning API is unavailable. A fresh interactive desktop walkthrough is not independently verified. No supplied real Projects or backups were used.

## Windows manual checklist

Use a disposable Project or a copy, never the supplied originals.

1. Open Characters in the sidebar, then Category settings. Confirm the selected Category, Types and defaults are correct. Rename it; the sidebar and Entry heading should update.
2. On an Entry, type a name/Field value, then use Entry settings → Delete Entry. Cancel once; confirm on the second attempt. Open Entries → View → Trash and restore it. Check its values, links and Spatial children.
3. Create a disposable Category with a Type, a default Field and several Entries. Delete it, choose another Category, review and confirm Type removal. Verify Entries and values remain and the old Type/default bindings are gone.
4. Open Backups and restore the recovery copy under a new name. Verify that the deleted Category, Types and original assignments are present in that copy.
5. In Timeline calendar settings, check the Era label help. In occurrence settings, check that an event page represents the event, while participants are linked under Entries.
