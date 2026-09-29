# Milestone 01 — Workspace UI cleanup

This records the PR #12 layout. PR #15's [sketchbook UI pass](MILESTONE_01_SKETCHBOOK_UI.md)
supersedes the visual theme, Entry title row, and creation disclosures while
retaining the save/draft protections described here.

This follows merged PR #11 and the user's request to reduce clutter. It applies
Concept V0.02's simple authoring path and the Home/editor/configuration separation
in Milestone 01 §§24–25. It reorganizes existing functionality without changing
Project storage, Field semantics, migrations, preferences, or dependencies.

## Everyday workflow

- Home prioritizes New Project and Open Project. Default folders are in
  **Settings**; **Restore a backup…** opens a separate dialog with native folder
  choosers and manual path entry. Technical path entry is expandable.
- The Project header shows the working name and save state. **Project settings**
  contains rename and Project information; **Backups** contains backup controls.
  Application-wide default folders remain in Settings on Home.
- Entries appear as readable rows with Category context. Creation stays inline,
  with optional Category/Type creation, and is expandable when browsing existing
  Entries. The first Entry form is available immediately in an empty Project.
- Entry editing prioritizes name and values. **Entry settings** contains
  Category/Type changes and the stable ID. Unapplied changes remain indicated
  after dismissing that dialog.
- **Add a field** reveals the local name/value form. Kind and scope live under
  **Field options**. **Manage fields** opens shared-definition controls separately
  from value editing. IDs appear in the selector only to disambiguate duplicate
  names.

## Preservation and accessibility

Native HTML dialogs contain keyboard focus and support Escape and Done. Closing
a dialog does not unmount its form, discard a draft, or cancel a submitted write.
Shared-definition drafts remain indicated and retain the existing navigation and
native-close protections. Save failures and preference/backup failures remain
visible even when an operation finishes after its dialog was dismissed.

The close confirmation is itself modal, so it can appear above an open settings
dialog. It offers Save and close for submittable changes, explicit discard, and
Cancel. Incomplete definition/structure forms still require finishing or explicit
discard. A single visible Project save indicator reflects the combined state.

The layout adapts to narrow windows, supports the system light/dark preference,
wraps long paths, and provides visible keyboard focus. No new product decision,
milestone feature, data migration, or persistent UI preference is introduced.

## Verification

Frontend tests cover hidden setup controls, restore-form preservation, Escape
and retained definition drafts, failures after dismissal, save-and-close, and the
existing save/recovery/navigation/default-folder contracts. The full frontend
typecheck, test, lint, formatting and production-build checks are required, along
with Rust formatting, tests, Clippy with warnings denied, Cargo check and Git
whitespace checks.

A separate local browser check uses synthetic in-memory Tauri responses. It
checks focus containment, Escape focus restoration, definition-draft preservation,
absence of horizontal overflow at 390 pixels, and browser errors. Screenshots
were inspected for Home, Settings, Project, Entry, shared Fields, narrow layout,
and dark appearance. This checks browser rendering and interaction; it does not
claim native Tauri window integration or real filesystem behavior was manually
tested. The supplied real Projects and backups were untouched.

No repository secret scanner or CodeQL workflow/CLI is available. A supplemental
credential-pattern scan of the changes is not equivalent to those tools.

## Windows manual review

Use a fresh test Project or disposable copy and launch from `app` with
`npm.cmd run tauri -- dev`.

1. Open Home Settings, change a default folder, dismiss, and verify the new
   default is immediately used. Check New/Open and Restore with both native
   folder selection and manual paths.
2. Open an Entry. Verify values are easy to reach and Category/Type and IDs are
   in Entry settings. Change Category/Type, dismiss without applying, and check
   that the pending change stays visible and navigation remains protected.
3. Add a field and use Field options. In Manage fields, begin a definition edit,
   press Escape, reopen, and verify the draft remains. Apply or cancel it.
4. Exercise Project rename and backup. Dismiss each dialog while a request is
   pending; success/failure must remain understandable after it settles.
5. Try Tab, Shift+Tab, Escape, and the OS close shortcut with a settings dialog
   open and unsaved changes. Save and close must wait for acknowledgement;
   discard must remain explicit. Test a failed save and retry.
6. Resize the window and check both system color schemes and increased display
   scaling. Paths, buttons, and dialog content should remain reachable.

Rich Text, Entry References, semantic conversions, Relationships, Story, Spatial,
Search and other milestone features remain outside this UI-only PR.
