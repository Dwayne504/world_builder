# Category defaults, Number units, and uninterrupted editing

This slice follows Concept V0.02, Milestone 01 sections 7–9, and approved
Architecture Proposal V2 sections 5–6. It addresses feedback after the
Relationships foundation. The user approved **Number + custom unit label, with
conversions deferred** on 2026-09-29.

## Delivered behavior

- **Number (optional unit)** is visible when adding a Field. Choose the unit when
  creating the definition; numeric values and the shared unit label are stored
  separately. For example, enter `8000000` and `tons`, or `25.5` and `gold crowns`.
  Units are arbitrary labels, so fictional measurements and currencies work.
- **Categories** opens a manager listing Categories and their Types. Create a
  Category or Type, optionally choose a parent Type, then configure default
  Fields for the Category or a particular Type. This works without any Entries.
  Reusing a Field binds the same stable definition ID instead of duplicating it.
- Defaults are available on existing and future matching Entries, including the
  currently open Entry. Category and ancestor-Type defaults combine. Each Entry
  owns its own values; blank defaults create no empty stored values. Removing a
  default preserves populated values and their units as detached Fields.
- **Entry settings → Create a Type in this Category → Create Type and select**
  lets an existing untyped Entry create a Type. **Apply Category / Type** explicitly
  commits the assignment. An unfinished name or assignment remains a protected
  draft; creating the Type alone does not assign it silently.
- Project Home groups Entries under collapsible Category headings with counts.
  Individual rows show the Entry name. Empty Category groups are omitted.
- Relationship-note autosave retains keyboard focus and newer text while an older
  write is pending. Field inputs and Clear controls also remain enabled during
  their own autosaves; Tab navigation does not lose focus when saving starts.
  Newer edits receive a subsequent save instead of being cleared by an older
  acknowledgement. Conflicting structural actions still wait.
- Manager drafts survive dialog dismissal and participate in Project/native-close
  protection. Pending writes must finish before closing. Failed writes retain
  their drafts and provide explicit reload/retry controls.

## Persistence and boundaries

Schema 5 adds optional `field_definition.unit`, restricted to nonblank Number
units. No existing numeric value changes; existing definitions have no unit.
The UI only chooses units on new definitions. A database trigger rejects unit
changes on populated definitions, preventing accidental reinterpretation.
There is no Short Text conversion or automatic conversion between units.

Template catalog/read and create/bind/unbind commands run through the existing
Project worker. Template writes check the Project revision and commit atomically;
only a Category or Type in that Project can be the provider. Template commands
cannot write Entry values. Unit/definition creation and its binding share one
transaction. Existing recovery snapshots, coordinated schema publication, and
Restore as Copy preserve the new metadata and internal IDs.

All test Projects are generated in disposable temporary directories. Supplied
real Projects and backups were not opened or modified. The preexisting local
Cargo.toml modification was preserved and excluded from the commit.

## Verification

Verified on Windows on 2026-09-29:

- `npm run typecheck`
- `npm test -- --run` — 114 tests; JSON reporter used for concise diagnostics;
  no React `act(...)` warnings
- `npm run lint` — no warnings
- `npm run format:check`
- `npm run build`
- `cargo fmt --check`
- `cargo test` — 127 tests
- `cargo clippy --all-targets -- -D warnings`
- `cargo check`
- `git diff --check`

Six new Rust integration tests cover defaults before Entries exist, independent
values, detach/rebind preservation, Category/ancestor-Type composition, stable
identity, provider/value/unit validation, rollback after an injected binding
failure, stale revisions, reopen, Restore as Copy, and a failed schema-4 upgrade
followed by recovery/retry with the original plain numeric value intact.

Frontend regressions cover Number/unit creation, Category and Type management,
default binding/removal, immediate adoption, grouped Entries, inline Type
assignment, hidden/failed drafts, native close during template creation, newer
values during delayed commits, and real keyboard Tab order in the Field panel.

Headless Microsoft Edge checks used synthetic Tauri command responses and delayed
save acknowledgements. They verified inline Type assignment, Category unit
defaults and immediate adoption, collapsible groups, keyboard Tab during Field
saves, continued relationship-note typing/caret retention through commits, modal
focus containment, and a 390-pixel dark layout. Screenshots were inspected;
no browser errors occurred. This does not replace a packaged Tauri/WebView2 check.

GitHub secret scanning and push protection are enabled; the repository had zero
open secret alerts at verification. No dedicated local secret-scanner CLI or
repository script is installed; staged additions are also checked for credential
patterns, personal paths, and forbidden artifacts. CodeQL default setup is
`not-configured`, with no CodeQL workflow or local CLI, so no CodeQL analysis is
claimed. GitHub CI status is reported separately on the PR.

## Windows manual checklist

Use a disposable Project or an explicit copy of a real `.wcproj` package.

1. Create a Number Field named Mass, unit `tons`, value `8000000`. Reopen the
   Project and check both. Try `25.5` with unit `gold crowns`. Invalid numeric
   input should retain the draft and show an error.
2. Open Categories, create Weapons and a Sword Type. Add Mass and Range defaults
   to Weapons; reuse one for Sword. Create two Entries and verify their empty
   defaults and independent values. A duplicate binding must not duplicate the
   Field. Check that existing/open Entries adopt newly added defaults.
3. Remove a default after filling it on one Entry. Its value and unit must remain;
   the unfilled Field should disappear from other matching Entries. Reuse the
   same definition and confirm the old value remains.
4. Open an untyped Entry, create/select Sword in Entry settings, then Apply
   Category / Type. Reopen and verify the assignment and appropriate defaults.
5. Fill several preset Fields using Tab (including their Clear buttons). Pause
   for autosave and continue typing. Clear during a save, then reopen and confirm
   the final values. Focus must stay on the current control.
6. Type a relationship note, pause long enough for autosave, then continue without
   clicking again. Check final text from both participants after reopening.
7. Dismiss a manager with an unfinished draft and reopen it. Test native window
   close during a save and after a failed save; pending/failed work must not be
   silently discarded. Confirm deliberate Cancel/discard remains available.
8. Collapse Category groups and inspect manager scrolling, keyboard focus, light
   and dark themes at the normal desktop window size.
9. Open a disposable schema-4 Project copy, create a backup after upgrading, and
   restore it as another copy. Confirm values, units, Types, and Relationships.

## Explicitly deferred

- Unit conversions, editing existing units, semantic Field-kind conversion, and
  importing numeric values from existing Short Text Fields.
- Thousands-separator/localized number parsing and formatting. Current entry uses
  ungrouped numbers with a dot for decimals; `8.000.000` is not accepted.
- Category/Type rename, retirement, deletion, and reparenting controls; standalone
  Category pages, Type filtering, Recent/Pinned Projects, and final navigation.
- Pre-filled default values, required Fields, custom Field ordering, relationship
  templates/projections, and all other milestone systems.

These defaults are optional Field availability, not copied initial values or a
completeness requirement. The broader Category manager and Ownership slice are
not claimed complete.
