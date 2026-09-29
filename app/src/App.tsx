import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import {
  AppCommandError,
  closeProject,
  createBackup,
  createCategory,
  createEntry,
  createProject,
  createType,
  changeEntryStructure,
  getPreferences,
  listCategories,
  listEntries,
  getEntry,
  listTypes,
  openProject,
  pickDirectory,
  previewPackagePath,
  resetPreferences,
  restoreBackupAsCopy,
  setDefaultBackupsDir,
  setDefaultProjectsDir,
} from "./api";
import type { Category, Entry, Preferences, ProjectSummary, SaveState, TypeDef } from "./types";
import { useProjectRename } from "./useProjectRename";
import type { SubmitOutcome } from "./useProjectRename";
import { useEntryName } from "./useEntryName";
import { useMutationCoordinator } from "./useMutationCoordinator";
import { decideClose, type CloseIntent } from "./closeDecision";
import { EntryFieldsPanel, type FieldsController } from "./EntryFieldsPanel";
import { EntryRelationshipsPanel } from "./EntryRelationshipsPanel";
import { Dialog } from "./Dialog";

function errorMessage(err: unknown): string {
  if (err instanceof AppCommandError) {
    return `${err.message} (${err.kind})`;
  }

  return err instanceof Error ? err.message : String(err);
}

/**
 * Understandable primary wording for open-Project lock failures. Raw
 * backend diagnostics stay visible through `errorMessage`; these messages
 * lead so the user is never greeted with implementation jargon.
 */
function openFailureMessage(err: unknown): string {
  if (err instanceof AppCommandError) {
    switch (err.kind) {
      case "lock_recovery_required":
        return (
          "This Project was not closed properly last time (for example after a crash or " +
          "power loss), so a leftover lock record is still present. The Project is not " +
          "currently open anywhere else, so you can recover it and open it now."
        );
      case "lock_held":
        return (
          "This Project is currently open in another Worldcrafter instance. Close it there " +
          "first; an active Project is never taken over."
        );
      case "lock_metadata_corrupt":
        return (
          "The Project's lock information is unreadable, so Worldcrafter cannot tell whether " +
          "the Project is safe to open. Nothing was changed. Close every Worldcrafter " +
          "instance, then remove the 'lock.json' file inside the Project package manually."
        );
      default:
        break;
    }
  }
  return errorMessage(err);
}

function openFailureDetails(err: unknown): string | null {
  const primary = openFailureMessage(err);
  const diagnostic = errorMessage(err);
  return diagnostic === primary ? null : diagnostic;
}

function isTauriWindow(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function closeWarningMessage(intent: CloseIntent): string {
  return intent === "native-window"
    ? "This Project has unsaved changes. Retry saving or discard before closing the app."
    : "This Project has unsaved changes. Retry saving or discard before closing the Project.";
}

function closeWarningActionLabel(intent: CloseIntent): string {
  return intent === "native-window"
    ? "Close app anyway (discard changes)"
    : "Close Project anyway (discard changes)";
}

function createFailureMessage(err: unknown): string {
  if (err instanceof AppCommandError && err.kind === "already_exists") {
    return (
      "A Project package already exists at that location with that name. Choose a " +
      "different working name, or pick another location, and try again."
    );
  }
  return errorMessage(err);
}

function preferencesFailureMessage(err: unknown): string {
  const suffix =
    "Your Project files are unaffected, and manual location entry remains available for " +
    "every operation.";
  if (err instanceof AppCommandError) {
    switch (err.kind) {
      case "preferences_corrupt":
        return `Your saved application preferences could not be read (the file appears corrupted). ${suffix}`;
      case "unsupported_preferences_version":
        return `Your saved application preferences were written by a different version of Worldcrafter and can't be read by this build. ${suffix}`;
      case "preferences_unavailable":
        return `Worldcrafter could not determine where to store application preferences on this system. ${suffix}`;
      default:
        break;
    }
  }
  return `Application preferences could not be loaded (${errorMessage(err)}). ${suffix}`;
}

function preferencesFailureKind(err: unknown): string | null {
  return err instanceof AppCommandError ? err.kind : null;
}

function canResetPreferences(kind: string | null): boolean {
  return kind === "preferences_corrupt";
}

function HomeScreen({ onOpened }: { onOpened: (project: ProjectSummary) => void }) {
  const [homeDialog, setHomeDialog] = useState<"settings" | "restore" | null>(null);
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [preferencesError, setPreferencesError] = useState<string | null>(null);
  const [preferencesErrorKind, setPreferencesErrorKind] = useState<string | null>(null);
  const [preferencesActionError, setPreferencesActionError] = useState<string | null>(null);
  const [resetBusy, setResetBusy] = useState(false);
  const [preferencesBusy, setPreferencesBusy] = useState(false);
  const [baseDir, setBaseDir] = useState("");
  const baseDirTouched = useRef(false);
  const preferencesRevision = useRef(0);
  const [newName, setNewName] = useState("");
  const [packagePreview, setPackagePreview] = useState<string | null>(null);
  const [createdSummary, setCreatedSummary] = useState<ProjectSummary | null>(null);
  const [openPath, setOpenPath] = useState("");
  const [backupPath, setBackupPath] = useState("");
  const [restoreDestination, setRestoreDestination] = useState("");
  const [restoreName, setRestoreName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [errorDetails, setErrorDetails] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Set when (and only when) the backend reports `lock_recovery_required`
  // for the current open path. The path input itself is never cleared, so
  // the user keeps what they typed after a failed open.
  const [recoveryPath, setRecoveryPath] = useState<string | null>(null);
  const openPathRef = useRef("");
  const openPathRevisionRef = useRef(0);

  useEffect(() => {
    let current = true;
    const revision = preferencesRevision.current;
    void getPreferences()
      .then((prefs) => {
        if (!current || revision !== preferencesRevision.current) return;
        setPreferences(prefs);
        setPreferencesError(null);
        setPreferencesErrorKind(null);
        if (!baseDirTouched.current && prefs.defaultProjectsDir && prefs.defaultProjectsDirExists) {
          setBaseDir(prefs.defaultProjectsDir);
        }
        return prefs;
      })
      .catch((err: unknown) => {
        if (!current || revision !== preferencesRevision.current) return;
        // A corrupt/unreadable preferences file must never be silently
        // swallowed: it is shown, with manual entry remaining available.
        setPreferences(null);
        setPreferencesError(preferencesFailureMessage(err));
        setPreferencesErrorKind(preferencesFailureKind(err));
        return null;
      });
    return () => {
      current = false;
    };
  }, []);

  useEffect(() => {
    setPackagePreview(null);
    if (!baseDir || !newName) {
      return;
    }
    let current = true;
    const timer = setTimeout(() => {
      void previewPackagePath(baseDir, newName)
        .then((path) => {
          if (current) setPackagePreview(path);
        })
        .catch(() => {
          if (current) setPackagePreview(null);
        });
    }, 150);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [baseDir, newName]);

  async function handleChooseProjectsLocation() {
    try {
      const picked = await pickDirectory(baseDir || preferences?.defaultProjectsDir);
      if (picked) {
        baseDirTouched.current = true;
        setBaseDir(picked);
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleCreate() {
    setBusy(true);
    setError(null);
    setErrorDetails(null);
    setCreatedSummary(null);
    try {
      const summary = await createProject(baseDir, newName);
      setCreatedSummary(summary);
      onOpened(summary);
    } catch (err) {
      setError(createFailureMessage(err));
      setErrorDetails(errorMessage(err) === createFailureMessage(err) ? null : errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleChooseOpenPath() {
    try {
      const picked = await pickDirectory(preferences?.defaultProjectsDir);
      if (picked) {
        openPathRef.current = picked;
        openPathRevisionRef.current += 1;
        setOpenPath(picked);
        setRecoveryPath(null);
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleOpen() {
    const attemptedPath = openPathRef.current;
    const pathRevision = openPathRevisionRef.current;
    setBusy(true);
    setError(null);
    setErrorDetails(null);
    setRecoveryPath(null);
    try {
      onOpened(await openProject(attemptedPath));
    } catch (err) {
      setError(openFailureMessage(err));
      setErrorDetails(openFailureDetails(err));
      const pathIsUnchanged =
        openPathRevisionRef.current === pathRevision && openPathRef.current === attemptedPath;
      setRecoveryPath(
        err instanceof AppCommandError && err.kind === "lock_recovery_required" && pathIsUnchanged
          ? attemptedPath
          : null,
      );
    } finally {
      setBusy(false);
    }
  }

  async function handleRecoverLock() {
    if (!recoveryPath || recoveryPath !== openPathRef.current) {
      setRecoveryPath(null);
      return;
    }
    const attemptedPath = recoveryPath;
    const pathRevision = openPathRevisionRef.current;
    setBusy(true);
    setError(null);
    setErrorDetails(null);
    setRecoveryPath(null);
    try {
      onOpened(await openProject(attemptedPath, true));
    } catch (err) {
      // A failed recovery must stay visible; the backend leaves the
      // Project package untouched.
      setError(openFailureMessage(err));
      setErrorDetails(openFailureDetails(err));
      const pathIsUnchanged =
        openPathRevisionRef.current === pathRevision && openPathRef.current === attemptedPath;
      setRecoveryPath(
        err instanceof AppCommandError && err.kind === "lock_recovery_required" && pathIsUnchanged
          ? attemptedPath
          : null,
      );
    } finally {
      setBusy(false);
    }
  }

  async function handleRestore() {
    setBusy(true);
    setError(null);
    setErrorDetails(null);
    try {
      onOpened(await restoreBackupAsCopy(backupPath, restoreDestination, restoreName || undefined));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleChooseRestoreFolder(kind: "backup" | "destination") {
    try {
      const picked = await pickDirectory(
        kind === "backup"
          ? backupPath || preferences?.defaultBackupsDir
          : restoreDestination || preferences?.defaultProjectsDir,
      );
      if (picked) {
        if (kind === "backup") setBackupPath(picked);
        else setRestoreDestination(picked);
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleChooseDefaultProjectsDir() {
    setPreferencesBusy(true);
    setPreferencesActionError(null);
    try {
      const picked = await pickDirectory(preferences?.defaultProjectsDir);
      if (!picked) return;
      preferencesRevision.current += 1;
      const updated = await setDefaultProjectsDir(picked);
      setPreferences(updated);
      // Immediately use the newly chosen default for New Project creation,
      // but only if the user has not already manually chosen a different
      // location for this operation.
      if (!baseDirTouched.current) {
        setBaseDir(updated.defaultProjectsDir ?? "");
      }
    } catch (err) {
      setPreferencesActionError(errorMessage(err));
    } finally {
      setPreferencesBusy(false);
    }
  }

  async function handleClearDefaultProjectsDir() {
    setPreferencesBusy(true);
    setPreferencesActionError(null);
    try {
      preferencesRevision.current += 1;
      setPreferences(await setDefaultProjectsDir(null));
      if (!baseDirTouched.current) setBaseDir("");
    } catch (err) {
      setPreferencesActionError(errorMessage(err));
    } finally {
      setPreferencesBusy(false);
    }
  }

  async function handleChooseDefaultBackupsDir() {
    setPreferencesBusy(true);
    setPreferencesActionError(null);
    try {
      const picked = await pickDirectory(preferences?.defaultBackupsDir);
      if (!picked) return;
      preferencesRevision.current += 1;
      setPreferences(await setDefaultBackupsDir(picked));
    } catch (err) {
      setPreferencesActionError(errorMessage(err));
    } finally {
      setPreferencesBusy(false);
    }
  }

  async function handleClearDefaultBackupsDir() {
    setPreferencesBusy(true);
    setPreferencesActionError(null);
    try {
      preferencesRevision.current += 1;
      setPreferences(await setDefaultBackupsDir(null));
    } catch (err) {
      setPreferencesActionError(errorMessage(err));
    } finally {
      setPreferencesBusy(false);
    }
  }

  async function handleResetPreferences() {
    setResetBusy(true);
    setPreferencesActionError(null);
    try {
      preferencesRevision.current += 1;
      const defaults = await resetPreferences();
      setPreferences(defaults);
      setPreferencesError(null);
      setPreferencesErrorKind(null);
      if (!baseDirTouched.current) setBaseDir("");
    } catch (err) {
      setPreferencesActionError(`Reset failed: ${errorMessage(err)}`);
    } finally {
      setResetBusy(false);
    }
  }

  return (
    <main className="container home-screen">
      <header className="app-header">
        <div>
          <p className="eyebrow">YOUR WORLDS, AT YOUR PACE</p>
          <h1>Worldcrafter</h1>
        </div>
        <button className="quiet-button" onClick={() => setHomeDialog("settings")}>
          Settings
        </button>
      </header>
      <p className="intro">
        A place for your characters, places, and ideas. Start small and build as you go.
      </p>
      {preferencesActionError && homeDialog !== "settings" && (
        <div role="alert" className="error-banner">
          <p>{preferencesActionError}</p>
          <button onClick={() => setHomeDialog("settings")}>Review settings</button>
        </div>
      )}
      {(preferencesError ||
        (preferences?.defaultProjectsDir && !preferences.defaultProjectsDirExists) ||
        (preferences?.defaultBackupsDir && !preferences.defaultBackupsDirExists)) && (
        <div role="alert" className="error-banner">
          <p>
            {preferencesError ??
              "A default folder is missing or inaccessible. Choose a location manually or update Settings."}
          </p>
          <button onClick={() => setHomeDialog("settings")}>Review settings</button>
        </div>
      )}
      {error && (
        <div role="alert" className="error-banner">
          <p>{error}</p>
          {errorDetails && (
            <details>
              <summary>Technical details</summary>
              <p>{errorDetails}</p>
            </details>
          )}
        </div>
      )}

      <Dialog
        open={homeDialog === "settings"}
        title="Application settings"
        onClose={() => setHomeDialog(null)}
      >
        <p className="muted">
          Choose where new Projects and backups are stored. Existing files stay where they are.
        </p>
        {preferencesError && (
          <div role="alert" className="error-banner">
            <p>Preferences need attention. Manual folder selection remains available.</p>
            {canResetPreferences(preferencesErrorKind) && (
              <>
                <p>
                  Reset saves the unreadable preferences as a diagnostic copy before restoring
                  defaults.
                </p>
                <button
                  disabled={resetBusy || preferencesBusy}
                  onClick={() => void handleResetPreferences()}
                >
                  Reset application preferences
                </button>
              </>
            )}
          </div>
        )}
        {homeDialog === "settings" && preferencesActionError && (
          <p role="alert" className="error-banner">
            {preferencesActionError}
          </p>
        )}
        <div className="preference-row">
          <span className="preference-label">Default Projects folder</span>
          <span className="preference-value">
            {preferences?.defaultProjectsDir ?? "Not set"}
            {preferences?.defaultProjectsDir && !preferences.defaultProjectsDirExists && (
              <span className="preference-warning"> (missing or inaccessible)</span>
            )}
          </span>
          <div className="row">
            <button
              disabled={preferencesBusy || resetBusy}
              onClick={() => void handleChooseDefaultProjectsDir()}
            >
              Choose…
            </button>
            {preferences?.defaultProjectsDir && (
              <button
                disabled={preferencesBusy || resetBusy}
                onClick={() => void handleClearDefaultProjectsDir()}
              >
                Clear
              </button>
            )}
          </div>
        </div>
        <div className="preference-row">
          <span className="preference-label">Default Backups folder</span>
          <span className="preference-value">
            {preferences?.defaultBackupsDir ?? "Not set"}
            {preferences?.defaultBackupsDir && !preferences.defaultBackupsDirExists && (
              <span className="preference-warning"> (missing or inaccessible)</span>
            )}
          </span>
          <div className="row">
            <button
              disabled={preferencesBusy || resetBusy}
              onClick={() => void handleChooseDefaultBackupsDir()}
            >
              Choose…
            </button>
            {preferences?.defaultBackupsDir && (
              <button
                disabled={preferencesBusy || resetBusy}
                onClick={() => void handleClearDefaultBackupsDir()}
              >
                Clear
              </button>
            )}
          </div>
        </div>
      </Dialog>

      <div className="home-grid">
        <section>
          <p className="eyebrow">START SOMETHING</p>
          <h2>New Project</h2>
          <label>
            Working name
            <input
              aria-label="new-project-name"
              value={newName}
              onChange={(e) => setNewName(e.currentTarget.value)}
              placeholder="Tortuga"
            />
          </label>
          <div className="preference-row">
            <span className="preference-label">Location</span>
            <span className="preference-value">{baseDir || "Choose a location"}</span>
            <button onClick={() => void handleChooseProjectsLocation()}>Choose location…</button>
          </div>
          {packagePreview && (
            <p className="package-preview">Will be created as: {packagePreview}</p>
          )}
          <details className="manual-path-diagnostics">
            <summary>Enter location manually</summary>
            <label>
              Location
              <input
                aria-label="new-project-location"
                value={baseDir}
                onChange={(e) => {
                  baseDirTouched.current = true;
                  setBaseDir(e.currentTarget.value);
                }}
                placeholder="Paste a folder path"
              />
            </label>
          </details>
          <button
            className="primary-button"
            disabled={busy || !baseDir || !newName}
            onClick={() => void handleCreate()}
          >
            Create Project
          </button>
          {createdSummary && (
            <p className="package-preview">Created at: {createdSummary.packagePath}</p>
          )}
        </section>

        <section>
          <p className="eyebrow">PICK UP WHERE YOU LEFT OFF</p>
          <h2>Open Project</h2>
          <p className="muted">Choose a Worldcrafter Project folder to continue.</p>
          <div className="row">
            <button disabled={busy} onClick={() => void handleChooseOpenPath()}>
              Browse for Project…
            </button>
          </div>
          {openPath && <p className="package-preview">{openPath}</p>}
          <details className="manual-path-diagnostics">
            <summary>Enter package path manually</summary>
            <label>
              Package path
              <input
                aria-label="open-project-path"
                value={openPath}
                onChange={(e) => {
                  const path = e.currentTarget.value;
                  openPathRef.current = path;
                  openPathRevisionRef.current += 1;
                  setOpenPath(path);
                  setRecoveryPath(null);
                }}
                placeholder="Paste a .wcproj folder path"
              />
            </label>
          </details>
          <button
            className="primary-button"
            disabled={busy || !openPath}
            onClick={() => void handleOpen()}
          >
            Open Project
          </button>
          {recoveryPath !== null && recoveryPath === openPath && (
            <div role="alert" className="error-banner">
              <p>
                Recovering removes only the leftover lock record; the Project's content is not
                modified. Use this only when you are sure no other Worldcrafter instance currently
                has this Project open.
              </p>
              <button disabled={busy} onClick={() => void handleRecoverLock()}>
                Recover lock and open Project
              </button>
            </div>
          )}
        </section>
      </div>
      <footer className="home-footer">
        <span className="muted">Your work stays on your computer.</span>
        <button className="quiet-button" onClick={() => setHomeDialog("restore")}>
          Restore a backup…
        </button>
      </footer>
      <Dialog
        open={homeDialog === "restore"}
        title="Restore Backup as Copy"
        onClose={() => setHomeDialog(null)}
      >
        <p className="muted">
          Create an independent Project from a backup. The original stays untouched.
        </p>
        {homeDialog === "restore" && error && (
          <p role="alert" className="error-banner">
            {error}
          </p>
        )}
        <label>
          Backup path
          <input
            aria-label="restore-backup-path"
            value={backupPath}
            onChange={(e) => setBackupPath(e.currentTarget.value)}
            placeholder="Choose a backup folder"
          />
        </label>
        <button disabled={busy} onClick={() => void handleChooseRestoreFolder("backup")}>
          Choose backup…
        </button>
        <label>
          Destination folder
          <input
            aria-label="restore-destination"
            value={restoreDestination}
            onChange={(e) => setRestoreDestination(e.currentTarget.value)}
          />
        </label>
        <button disabled={busy} onClick={() => void handleChooseRestoreFolder("destination")}>
          Choose destination…
        </button>
        <label>
          New working name (optional)
          <input
            aria-label="restore-new-name"
            value={restoreName}
            onChange={(e) => setRestoreName(e.currentTarget.value)}
          />
        </label>
        <button
          disabled={busy || !backupPath || !restoreDestination}
          onClick={() => void handleRestore()}
        >
          Restore as Copy
        </button>
      </Dialog>
    </main>
  );
}

function saveStateLabel(state: string): string {
  switch (state) {
    case "saved":
      return "Saved";
    case "dirty":
      return "Pending";
    case "saving":
      return "Saving…";
    case "failed":
      return "Failed to save";
    default:
      return state;
  }
}

interface EntrySaveController {
  state: SaveState;
  submit: () => Promise<SubmitOutcome>;
  canSubmit: boolean;
}

type MutationCoordinator = ReturnType<typeof useMutationCoordinator>;

function EntryEditor({
  projectId,
  initialEntry,
  categories,
  onChanged,
  onClose,
  onNavigate,
  onController,
  mutations,
}: {
  projectId: string;
  initialEntry: Entry;
  categories: Category[];
  onChanged: (entry: Entry) => void;
  onClose: () => void;
  onNavigate: (id: string) => void;
  onController: (controller: EntrySaveController) => void;
  mutations: MutationCoordinator;
}) {
  const editor = useEntryName(projectId, initialEntry);
  const [entrySettingsOpen, setEntrySettingsOpen] = useState(false);
  const { submit: submitName, currentEntry } = editor;
  const { isPending: isStructurePending, waitForPending: waitForStructure } = mutations;
  const committedRevision = useRef(initialEntry.globalRevision);
  committedRevision.current = Math.max(committedRevision.current, editor.entry.globalRevision);
  const getRevision = useCallback(() => committedRevision.current, []);
  const relationshipsController = useRef<FieldsController | null>(null);
  const [relationshipsState, setRelationshipsState] = useState<SaveState>("saved");
  const [relationshipsCanSubmit, setRelationshipsCanSubmit] = useState(true);
  const receiveRelationships = useCallback((controller: FieldsController) => {
    relationshipsController.current = controller;
    setRelationshipsState(controller.state);
    setRelationshipsCanSubmit(controller.canSubmit);
  }, []);
  const structureDirtyRef = useRef(false);
  const fieldsController = useRef<FieldsController | null>(null);
  const [fieldsState, setFieldsState] = useState<SaveState>("saved");
  const [fieldsCanSubmit, setFieldsCanSubmit] = useState(true);
  const receiveFields = useCallback((controller: FieldsController) => {
    fieldsController.current = controller;
    setFieldsState(controller.state);
    setFieldsCanSubmit(controller.canSubmit);
  }, []);
  const submit = useCallback(
    async (applyingStructure = false): Promise<SubmitOutcome> => {
      const relationshipOutcome = await (relationshipsController.current?.submit() ??
        Promise.resolve({ kind: "no-op" } as SubmitOutcome));
      if (relationshipOutcome.kind === "failed" || relationshipOutcome.kind === "committed-stale")
        return relationshipOutcome;
      const fieldOutcome = await (fieldsController.current?.submit() ??
        Promise.resolve({ kind: "no-op" } as SubmitOutcome));
      if (fieldOutcome.kind === "failed" || fieldOutcome.kind === "committed-stale")
        return fieldOutcome;
      if (!applyingStructure && isStructurePending() && !(await waitForStructure()))
        return { kind: "failed" };
      if (structureDirtyRef.current && !applyingStructure) return { kind: "failed" };
      return submitName();
    },
    [submitName, isStructurePending, waitForStructure],
  );
  const [types, setTypes] = useState<TypeDef[]>([]);
  const [categoryId, setCategoryId] = useState(editor.entry.categoryId);
  const [typeId, setTypeId] = useState(editor.entry.typeId ?? "");
  const [structureError, setStructureError] = useState<string | null>(null);
  const [structureTypeChosen, setStructureTypeChosen] = useState(true);
  const onChangedRef = useRef(onChanged);
  onChangedRef.current = onChanged;
  const receiveRevision = useCallback(
    (revision: number) => {
      committedRevision.current = Math.max(committedRevision.current, revision);
      onChangedRef.current({ ...currentEntry(), globalRevision: committedRevision.current });
    },
    [currentEntry],
  );
  const structureDirty =
    categoryId !== editor.entry.categoryId || typeId !== (editor.entry.typeId ?? "");
  structureDirtyRef.current = structureDirty;
  const combinedEntryState: SaveState =
    editor.saveState === "saving" ||
    mutations.state === "saving" ||
    fieldsState === "saving" ||
    relationshipsState === "saving"
      ? "saving"
      : editor.saveState === "failed" ||
          mutations.state === "failed" ||
          fieldsState === "failed" ||
          relationshipsState === "failed"
        ? "failed"
        : editor.saveState === "dirty" ||
            structureDirty ||
            fieldsState === "dirty" ||
            relationshipsState === "dirty"
          ? "dirty"
          : "saved";

  useEffect(() => {
    onController({
      state: combinedEntryState,
      submit,
      canSubmit: !structureDirty && fieldsCanSubmit && relationshipsCanSubmit,
    });
  }, [
    combinedEntryState,
    onController,
    structureDirty,
    fieldsCanSubmit,
    relationshipsCanSubmit,
    submit,
  ]);

  useEffect(() => {
    onChangedRef.current(editor.entry);
  }, [editor.entry]);

  useEffect(() => {
    let current = true;
    setTypes([]);
    void listTypes(projectId, categoryId)
      .then((nextTypes) => {
        if (current) setTypes(nextTypes);
      })
      .catch((error) => {
        if (current) setStructureError(errorMessage(error));
      });
    return () => {
      current = false;
    };
  }, [categoryId, projectId]);

  async function saveStructure() {
    const submittedCategoryId = categoryId;
    const submittedTypeId = typeId;
    const outcome = await mutations.run(
      async () => {
        const nameOutcome = await submit(true);
        if (nameOutcome.kind === "failed") {
          throw new Error("Entry name must save before applying Category / Type.");
        }
        if (nameOutcome.kind === "committed-stale") {
          throw new Error("Entry name changed while applying Category / Type.");
        }
        const current = editor.currentEntry();
        return changeEntryStructure(
          projectId,
          current.id,
          submittedCategoryId,
          submittedTypeId || undefined,
          current.revision,
        );
      },
      (updated) => {
        structureDirtyRef.current = false;
        editor.replaceEntry(updated);
        setCategoryId(updated.categoryId);
        setTypeId(updated.typeId ?? "");
        setStructureTypeChosen(true);
        onController({ state: "saved", submit, canSubmit: true });
        onChangedRef.current(updated);
        setStructureError(null);
      },
    );
    if (outcome.kind === "failed") {
      setStructureError(outcome.errorMessage);
    }
  }

  return (
    <section className="entry-editor">
      <div className="section-heading">
        <div>
          <p className="eyebrow">ENTRY</p>
          <h2>{editor.entry.displayName}</h2>
        </div>
        <div className="row">
          <button className="quiet-button" onClick={() => setEntrySettingsOpen(true)}>
            Entry settings
          </button>
          <button className="quiet-button" onClick={onClose}>
            Back to Entries
          </button>
        </div>
      </div>
      <label>
        Name (optional)
        <input
          aria-label="entry-name"
          disabled={mutations.state === "saving" || relationshipsState === "saving"}
          value={editor.draftName}
          onChange={(event) => editor.onChangeDraft(event.currentTarget.value)}
        />
      </label>
      <span data-testid="entry-save-state" className="sr-only">
        {saveStateLabel(combinedEntryState)}
      </span>
      {editor.errorMessage && <p role="alert">{editor.errorMessage}</p>}
      {structureDirty && !entrySettingsOpen && (
        <p className="field-note">
          Category / Type changes are not applied.{" "}
          <button className="quiet-button" onClick={() => setEntrySettingsOpen(true)}>
            Review Entry settings
          </button>
        </p>
      )}
      {structureError && !entrySettingsOpen && (
        <div role="alert" className="error-banner">
          <p>{structureError}</p>
          <button onClick={() => setEntrySettingsOpen(true)}>Review Entry settings</button>
        </div>
      )}
      <Dialog
        open={entrySettingsOpen}
        title="Entry settings"
        onClose={() => setEntrySettingsOpen(false)}
      >
        <p className="muted">
          Organize this Entry and choose its available fields. Existing values are preserved.
        </p>
        <label>
          Category
          <select
            aria-label="entry-category"
            disabled={mutations.state === "saving" || relationshipsState === "saving"}
            value={categoryId}
            onChange={(event) => {
              setTypes([]);
              setCategoryId(event.currentTarget.value);
              setStructureTypeChosen(
                event.currentTarget.value === editor.entry.categoryId || !typeId,
              );
            }}
          >
            {categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Type (optional)
          <select
            aria-label="entry-type"
            disabled={mutations.state === "saving" || relationshipsState === "saving"}
            value={typeId}
            onChange={(event) => {
              setTypeId(event.currentTarget.value);
              setStructureTypeChosen(true);
            }}
          >
            <option value="">No Type</option>
            {typeId && !types.some((type) => type.id === typeId) && (
              <option value={typeId} disabled>
                Incompatible current Type — choose explicitly
              </option>
            )}
            {types.map((type) => (
              <option key={type.id} value={type.id}>
                {type.name}
              </option>
            ))}
          </select>
        </label>
        <div className="row">
          <button
            disabled={
              mutations.state === "saving" ||
              relationshipsState === "saving" ||
              !structureDirty ||
              (categoryId !== editor.entry.categoryId && !structureTypeChosen)
            }
            onClick={() => void saveStructure()}
          >
            Apply Category / Type
          </button>
        </div>
        {entrySettingsOpen && structureError && <p role="alert">{structureError}</p>}
        <details className="technical-details">
          <summary>Entry information</summary>
          <p className="package-preview">Entry ID: {editor.entry.id}</p>
        </details>
      </Dialog>
      <EntryFieldsPanel
        projectId={projectId}
        entry={editor.entry}
        disabled={
          mutations.state === "saving" ||
          editor.saveState === "saving" ||
          relationshipsState === "saving"
        }
        onController={receiveFields}
        onRevision={receiveRevision}
        getRevision={getRevision}
      />
      <EntryRelationshipsPanel
        projectId={projectId}
        entryId={editor.entry.id}
        categories={categories}
        disabled={
          mutations.state === "saving" ||
          editor.saveState !== "saved" ||
          fieldsState !== "saved" ||
          structureDirty
        }
        onController={receiveRelationships}
        onRevision={receiveRevision}
        getRevision={getRevision}
        onNavigate={onNavigate}
      />
    </section>
  );
}

function EntryWorkflow({
  projectId,
  onController,
  onGlobalRevision,
  mutations,
}: {
  projectId: string;
  onController: (controller: EntrySaveController | null) => void;
  onGlobalRevision: (revision: number) => void;
  mutations: MutationCoordinator;
}) {
  const [categories, setCategories] = useState<Category[]>([]);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [types, setTypes] = useState<TypeDef[]>([]);
  const [selected, setSelected] = useState<Entry | null>(null);
  const [draftName, setDraftName] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [typeId, setTypeId] = useState("");
  const [newCategoryName, setNewCategoryName] = useState("");
  const [newTypeName, setNewTypeName] = useState("");
  const [showCategoryCreator, setShowCategoryCreator] = useState(false);
  const [showTypeCreator, setShowTypeCreator] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const controllerRef = useRef<EntrySaveController | null>(null);
  const [pendingEntry, setPendingEntry] = useState<Entry | null>(null);
  const [pendingBack, setPendingBack] = useState(false);
  const [controllerState, setControllerState] = useState<SaveState>("saved");
  const [controllerCanSubmit, setControllerCanSubmit] = useState(true);

  const refresh = useCallback(async () => {
    const [nextCategories, nextEntries] = await Promise.all([
      listCategories(projectId),
      listEntries(projectId),
    ]);
    setCategories(nextCategories);
    setEntries(nextEntries);
    if (!categoryId) {
      setCategoryId(nextCategories.find((item) => item.isUncategorized)?.id ?? "");
    }
  }, [categoryId, projectId]);

  useEffect(() => {
    void refresh().catch((reason) => setError(errorMessage(reason)));
  }, [refresh]);

  useEffect(() => {
    let current = true;
    setTypes([]);
    if (!categoryId) {
      return () => {
        current = false;
      };
    }
    void listTypes(projectId, categoryId)
      .then((nextTypes) => {
        if (current) setTypes(nextTypes);
      })
      .catch((reason) => {
        if (current) setError(errorMessage(reason));
      });
    return () => {
      current = false;
    };
  }, [categoryId, projectId]);

  const receiveController = useCallback(
    (controller: EntrySaveController) => {
      controllerRef.current = controller;
      setControllerState(controller.state);
      setControllerCanSubmit(controller.canSubmit);
      onController(controller);
    },
    [onController],
  );

  function closeEditor() {
    if (mutations.isPending()) {
      setPendingEntry(null);
      setPendingBack(true);
      void mutations.waitForPending().then((successful) => {
        if (successful) {
          controllerRef.current = null;
          onController(null);
          setSelected(null);
        } else {
          setError(mutations.currentError() ?? "Structural save failed.");
        }
      });
      return;
    }
    if (controllerRef.current?.state !== "saved") {
      setPendingEntry(null);
      setPendingBack(true);
      setError("This Entry has unsaved changes. Save or discard them before navigating.");
      return;
    }
    controllerRef.current = null;
    onController(null);
    setSelected(null);
  }

  function openEntry(entry: Entry) {
    if (mutations.isPending()) {
      setPendingEntry(entry);
      setPendingBack(false);
      void mutations.waitForPending().then((successful) => {
        if (successful) {
          setPendingEntry(null);
          setSelected(entry);
        } else {
          setError(mutations.currentError() ?? "Structural save failed.");
        }
      });
      return;
    }
    if (selected && controllerRef.current?.state !== "saved") {
      setPendingEntry(entry);
      setPendingBack(false);
      setError("This Entry has unsaved changes. Save or discard them before navigating.");
      return;
    }
    setSelected(entry);
  }

  async function saveAndNavigate() {
    const outcome = await controllerRef.current?.submit();
    if (outcome?.kind === "committed" || outcome?.kind === "no-op") {
      const destination = pendingEntry;
      const goBack = pendingBack;
      setPendingEntry(null);
      setPendingBack(false);
      setError(null);
      if (destination) setSelected(destination);
      else if (goBack) {
        controllerRef.current = null;
        onController(null);
        setSelected(null);
      }
    }
  }

  function discardAndNavigate() {
    const destination = pendingEntry;
    const goBack = pendingBack;
    setPendingEntry(null);
    setPendingBack(false);
    setError(null);
    controllerRef.current = null;
    onController(null);
    setSelected(goBack ? null : destination);
  }

  async function addCategory() {
    const outcome = await mutations.run(
      () => createCategory(projectId, newCategoryName),
      (category) => {
        onGlobalRevision(category.globalRevision);
        setCategories((items) => [...items, category]);
        setCategoryId(category.id);
        setTypeId("");
        setNewCategoryName("");
        setShowCategoryCreator(false);
      },
    );
    if (outcome.kind === "failed") {
      setError(outcome.errorMessage);
    }
  }

  async function addType() {
    const outcome = await mutations.run(
      () => createType(projectId, categoryId, newTypeName),
      (type) => {
        onGlobalRevision(type.globalRevision);
        setTypes((items) => [...items, type]);
        setTypeId(type.id);
        setNewTypeName("");
        setShowTypeCreator(false);
      },
    );
    if (outcome.kind === "failed") {
      setError(outcome.errorMessage);
    }
  }

  async function addEntry() {
    const outcome = await mutations.run(
      () =>
        createEntry(
          projectId,
          draftName || undefined,
          categoryId || undefined,
          typeId || undefined,
        ),
      (entry) => {
        setEntries((items) => [...items, entry]);
        onGlobalRevision(entry.globalRevision);
        setDraftName("");
        setSelected(entry);
      },
    );
    if (outcome.kind === "failed") {
      setError(outcome.errorMessage);
    }
  }

  if (selected) {
    return (
      <>
        {error && (
          <div role="alert">
            <p>{error}</p>
            {(pendingEntry || pendingBack) && (
              <div className="row">
                {controllerCanSubmit && (
                  <button onClick={() => void saveAndNavigate()}>Save and continue</button>
                )}
                <button disabled={controllerState === "saving"} onClick={discardAndNavigate}>
                  Discard and continue
                </button>
                <button
                  onClick={() => {
                    setPendingEntry(null);
                    setPendingBack(false);
                    setError(null);
                  }}
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        )}
        <EntryEditor
          key={selected.id}
          projectId={projectId}
          initialEntry={selected}
          categories={categories}
          mutations={mutations}
          onController={receiveController}
          onClose={() => {
            closeEditor();
            void refresh();
          }}
          onNavigate={(id) => {
            void getEntry(projectId, id)
              .then(openEntry)
              .catch((reason) => setError(errorMessage(reason)));
          }}
          onChanged={(updated) => {
            onGlobalRevision(updated.globalRevision);
            setSelected(updated);
            setEntries((items) => items.map((item) => (item.id === updated.id ? updated : item)));
          }}
        />
      </>
    );
  }

  return (
    <section className="entries-panel">
      <div className="section-heading">
        <div>
          <p className="eyebrow">WORLD MATERIAL</p>
          <h2>Entries</h2>
        </div>
      </div>
      <p className="muted">Characters, places, objects, or an idea without a name yet.</p>
      {entries.length === 0 && (
        <p className="empty-state">
          Your world starts with one idea. Create your first Entry below.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <ul className="entry-list">
        {entries.map((entry) => (
          <li key={entry.id}>
            <button disabled={mutations.state === "saving"} onClick={() => openEntry(entry)}>
              <span>{entry.displayName}</span>
              <span aria-hidden="true" className="entry-category-label">
                {categories.find((category) => category.id === entry.categoryId)?.name}
              </span>
            </button>
          </li>
        ))}
      </ul>
      <details className="disclosure create-entry" open={entries.length === 0 ? true : undefined}>
        <summary>Create Entry</summary>
        <label>
          Name (optional)
          <input
            aria-label="new-entry-name"
            value={draftName}
            onChange={(event) => setDraftName(event.currentTarget.value)}
          />
        </label>
        <label>
          Category
          <select
            aria-label="new-entry-category"
            disabled={mutations.state === "saving"}
            value={categoryId}
            onChange={(event) => {
              setTypes([]);
              setCategoryId(event.currentTarget.value);
              setTypeId("");
            }}
          >
            {categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>
        </label>
        <button
          disabled={mutations.state === "saving"}
          onClick={() => setShowCategoryCreator(true)}
        >
          Create Category inline
        </button>
        {showCategoryCreator && (
          <fieldset className="inline-creator">
            <legend>New Category</legend>
            <label>
              Category name
              <input
                aria-label="inline-category-name"
                value={newCategoryName}
                onChange={(event) => setNewCategoryName(event.currentTarget.value)}
              />
            </label>
            <div className="row">
              <button
                disabled={!newCategoryName.trim() || mutations.state === "saving"}
                onClick={() => void addCategory()}
              >
                Add Category
              </button>
              <button
                disabled={mutations.state === "saving"}
                onClick={() => {
                  setNewCategoryName("");
                  setShowCategoryCreator(false);
                }}
              >
                Cancel
              </button>
            </div>
          </fieldset>
        )}
        <label>
          Type (optional)
          <select
            aria-label="new-entry-type"
            disabled={mutations.state === "saving"}
            value={typeId}
            onChange={(event) => setTypeId(event.currentTarget.value)}
          >
            <option value="">No Type</option>
            {types.map((type) => (
              <option key={type.id} value={type.id}>
                {type.name}
              </option>
            ))}
          </select>
        </label>
        <button
          disabled={!categoryId || mutations.state === "saving"}
          onClick={() => setShowTypeCreator(true)}
        >
          Create Type inline
        </button>
        {showTypeCreator && (
          <fieldset className="inline-creator">
            <legend>New Type</legend>
            <label>
              Type name
              <input
                aria-label="inline-type-name"
                value={newTypeName}
                onChange={(event) => setNewTypeName(event.currentTarget.value)}
              />
            </label>
            <div className="row">
              <button
                disabled={!newTypeName.trim() || mutations.state === "saving"}
                onClick={() => void addType()}
              >
                Add Type
              </button>
              <button
                disabled={mutations.state === "saving"}
                onClick={() => {
                  setNewTypeName("");
                  setShowTypeCreator(false);
                }}
              >
                Cancel
              </button>
            </div>
          </fieldset>
        )}
        <button disabled={mutations.state === "saving"} onClick={() => void addEntry()}>
          Create Entry
        </button>
      </details>
    </section>
  );
}

function ProjectScreen({ project, onClosed }: { project: ProjectSummary; onClosed: () => void }) {
  const [projectDialog, setProjectDialog] = useState<"settings" | "backup" | null>(null);
  const rename = useProjectRename(project);
  const mutations = useMutationCoordinator();
  const {
    state: mutationState,
    waitForPending: waitForStructuralMutation,
    currentError: currentMutationError,
    isPending: isStructuralMutationPending,
  } = mutations;
  const { submit: renameSubmit } = rename;
  const [backupDir, setBackupDir] = useState("");
  const backupDirTouched = useRef(false);
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const [pendingCloseIntent, setPendingCloseIntent] = useState<CloseIntent | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const approvedNativeClose = useRef(false);
  const entryControllerRef = useRef<EntrySaveController | null>(null);
  const [entrySaveState, setEntrySaveState] = useState<SaveState>("saved");
  const saveStateRef = useRef<SaveState>(rename.saveState);
  const closeInFlight = useRef<Promise<void> | null>(null);
  const waitingCloseRef = useRef<Promise<void> | null>(null);
  const waitingCloseIntentRef = useRef<CloseIntent | null>(null);
  const nativeWindowCloseRequested = useRef(false);
  const combinedSaveState: SaveState = [rename.saveState, entrySaveState, mutationState].includes(
    "saving",
  )
    ? "saving"
    : [rename.saveState, entrySaveState, mutationState].includes("failed")
      ? "failed"
      : [rename.saveState, entrySaveState].includes("dirty")
        ? "dirty"
        : "saved";
  saveStateRef.current = combinedSaveState;

  const receiveEntryController = useCallback((controller: EntrySaveController | null) => {
    entryControllerRef.current = controller;
    setEntrySaveState(controller?.state ?? "saved");
  }, []);

  useEffect(() => {
    let current = true;
    void getPreferences()
      .then((prefs) => {
        if (!current) return;
        if (!backupDirTouched.current && prefs.defaultBackupsDir && prefs.defaultBackupsDirExists) {
          setBackupDir(prefs.defaultBackupsDir);
        }
        if (prefs.defaultBackupsDir && !prefs.defaultBackupsDirExists) {
          setBackupStatus(
            "The default Backups folder is missing or inaccessible. Choose another location.",
          );
        }
      })
      .catch((err: unknown) => {
        if (current) setBackupStatus(preferencesFailureMessage(err));
      });
    return () => {
      current = false;
    };
  }, []);

  async function handleChooseBackupDir() {
    try {
      const picked = await pickDirectory(backupDir);
      if (picked) {
        backupDirTouched.current = true;
        setBackupDir(picked);
      }
    } catch (err) {
      setBackupStatus(errorMessage(err));
    }
  }

  async function handleCreateBackup() {
    setBusy(true);
    setBackupStatus(null);
    try {
      const path = await createBackup(project.projectId, backupDir);
      setBackupStatus(`Backup created at ${path}`);
    } catch (err) {
      setBackupStatus(`Backup failed: ${errorMessage(err)}`);
    } finally {
      setBusy(false);
    }
  }

  const closeAfterBackend = useCallback(
    (intent: CloseIntent): Promise<void> => {
      if (intent === "native-window") {
        nativeWindowCloseRequested.current = true;
      }
      if (closeInFlight.current) {
        return closeInFlight.current;
      }
      const close = (async () => {
        let closed = false;
        setBusy(true);
        setCloseError(null);
        try {
          await closeProject(project.projectId);
          const shouldExitWindow = nativeWindowCloseRequested.current;
          closed = true;
          onClosed();
          if (shouldExitWindow && isTauriWindow()) {
            approvedNativeClose.current = true;
            await getCurrentWindow().close();
          }
        } catch (err) {
          setCloseError(errorMessage(err));
        } finally {
          if (!closed) {
            nativeWindowCloseRequested.current = false;
          }
          setBusy(false);
        }
      })().finally(() => {
        nativeWindowCloseRequested.current = false;
        closeInFlight.current = null;
      });
      closeInFlight.current = close;
      return close;
    },
    [onClosed, project.projectId],
  );

  const waitForSaveThenClose = useCallback(
    (intent: CloseIntent): Promise<void> => {
      waitingCloseIntentRef.current =
        waitingCloseIntentRef.current === "native-window" || intent === "native-window"
          ? "native-window"
          : "project";
      if (waitingCloseRef.current) {
        return waitingCloseRef.current;
      }
      // The save already in flight cannot be cancelled, so we never treat it
      // as discardable: wait for it to settle, then close only if the
      // explicit outcome confirms the currently displayed draft committed.
      // Reading `saveState` back out of React state here would race the
      // render that applies it, so branch on the promise's own result
      // instead -- this also correctly refuses to close when a newer draft
      // was typed while the older save was still in flight.
      const waiting = Promise.all([
        renameSubmit(),
        entryControllerRef.current?.submit() ?? Promise.resolve({ kind: "no-op" } as SubmitOutcome),
        waitForStructuralMutation(),
      ])
        .then(([renameOutcome, entryOutcome, structuralSuccessful]) => {
          if (
            structuralSuccessful &&
            [renameOutcome, entryOutcome].every(
              (outcome) => outcome.kind === "committed" || outcome.kind === "no-op",
            )
          ) {
            return closeAfterBackend(waitingCloseIntentRef.current ?? intent);
          }
          if (!structuralSuccessful) {
            setCloseError(currentMutationError() ?? "Structural save failed.");
          }
          return undefined;
        })
        .finally(() => {
          waitingCloseRef.current = null;
          waitingCloseIntentRef.current = null;
        });
      waitingCloseRef.current = waiting;
      return waiting;
    },
    [renameSubmit, closeAfterBackend, currentMutationError, waitForStructuralMutation],
  );

  const requestClose = useCallback(
    (intent: CloseIntent): void => {
      setProjectDialog(null);
      setCloseError(null);
      if (isStructuralMutationPending()) {
        setPendingCloseIntent(null);
        void waitForSaveThenClose(intent);
        return;
      }
      const decision = decideClose(saveStateRef.current, intent);
      switch (decision) {
        case "close-project":
        case "close-native-window":
          setPendingCloseIntent(null);
          void closeAfterBackend(intent);
          return;
        case "wait-for-save-project":
        case "wait-for-save-native-window":
          setPendingCloseIntent(null);
          void waitForSaveThenClose(intent);
          return;
        case "confirm-unsaved-project":
        case "confirm-unsaved-native-window":
          setPendingCloseIntent((current) =>
            current === "native-window" || intent === "native-window" ? "native-window" : "project",
          );
      }
    },
    [closeAfterBackend, isStructuralMutationPending, waitForSaveThenClose],
  );
  const requestCloseRef = useRef(requestClose);
  requestCloseRef.current = requestClose;

  useEffect(() => {
    if (rename.saveState === "saved" && entrySaveState === "saved" && mutationState === "saved") {
      setPendingCloseIntent(null);
    }
  }, [entrySaveState, mutationState, rename.saveState]);

  function handleClose() {
    requestClose("project");
  }

  function handleForceCloseDiscarding() {
    if (!pendingCloseIntent) {
      return;
    }
    const intent = pendingCloseIntent;
    setPendingCloseIntent(null);
    if (saveStateRef.current === "saving") {
      void waitForSaveThenClose(intent);
      return;
    }
    void closeAfterBackend(intent);
  }

  useEffect(() => {
    if (!isTauriWindow()) {
      return;
    }
    const window = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void window
      .onCloseRequested((event) => {
        if (approvedNativeClose.current) {
          return;
        }
        event.preventDefault();
        requestCloseRef.current("native-window");
      })
      .then((listener) => {
        if (disposed) {
          listener();
          return;
        }
        unlisten = listener;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return (
    <main className="container project-screen">
      <header className="app-header">
        <div>
          <p className="eyebrow">WORLDCRAFTER</p>
          <h1>{rename.committedName}</h1>
        </div>
        <nav className="toolbar" aria-label="Project actions">
          <span
            data-testid="save-state"
            role="status"
            className={`save-state save-state-${combinedSaveState}`}
          >
            {saveStateLabel(combinedSaveState)}
          </span>
          <button className="quiet-button" onClick={() => setProjectDialog("backup")}>
            Backups
          </button>
          <button className="quiet-button" onClick={() => setProjectDialog("settings")}>
            Project settings
          </button>
          <button className="quiet-button" disabled={busy} onClick={handleClose}>
            Close Project
          </button>
        </nav>
      </header>
      {backupStatus && projectDialog !== "backup" && (
        <div role="status" className="backup-notice">
          <p>{backupStatus.startsWith("Backup created at ") ? "Backup created." : backupStatus}</p>
          <button className="quiet-button" onClick={() => setProjectDialog("backup")}>
            Review backup
          </button>
        </div>
      )}
      {rename.errorMessage && projectDialog !== "settings" && (
        <div role="alert" className="error-banner">
          <p>{rename.errorMessage}</p>
          <button onClick={() => setProjectDialog("settings")}>Review Project name</button>
        </div>
      )}
      <Dialog
        open={projectDialog === "settings"}
        title="Project settings"
        onClose={() => setProjectDialog(null)}
      >
        <p className="muted">
          Rename this Project without moving its files. Changes save automatically.
        </p>
        <label>
          Working name
          <input
            aria-label="project-working-name"
            value={rename.draftName}
            onChange={(e) => rename.onChangeDraft(e.currentTarget.value)}
          />
        </label>
        <div className="row">
          <button
            disabled={rename.saveState === "saving" || rename.draftName === rename.committedName}
            onClick={() => rename.submit()}
          >
            {rename.saveState === "failed" ? "Retry save" : "Save"}
          </button>
          <span className={`save-state save-state-${rename.saveState}`}>
            {saveStateLabel(rename.saveState)}
          </span>
        </div>
        {projectDialog === "settings" && rename.errorMessage && (
          <p role="alert" className="error-banner">
            {rename.errorMessage}
          </p>
        )}

        <details className="technical-details">
          <summary>Project information</summary>
          <dl>
            <dt>Project ID</dt>
            <dd data-testid="project-id">{project.projectId}</dd>
            <dt>Location</dt>
            <dd>{project.packagePath}</dd>
            <dt>Schema version</dt>
            <dd>{project.schemaVersion}</dd>
          </dl>
        </details>
      </Dialog>

      <EntryWorkflow
        projectId={project.projectId}
        onController={receiveEntryController}
        onGlobalRevision={rename.updateRevision}
        mutations={mutations}
      />

      <Dialog
        open={projectDialog === "backup"}
        title="Backups"
        onClose={() => setProjectDialog(null)}
      >
        <p className="muted">
          Save a separate snapshot of the committed Project. Restore it from Home whenever you need
          a copy.
        </p>
        <label>
          Backup destination folder
          <input
            aria-label="backup-destination"
            value={backupDir}
            onChange={(e) => {
              backupDirTouched.current = true;
              setBackupDir(e.currentTarget.value);
            }}
          />
        </label>
        <div className="row">
          <button onClick={() => void handleChooseBackupDir()}>Choose location…</button>
          <button disabled={busy || !backupDir} onClick={() => void handleCreateBackup()}>
            Create Manual Backup
          </button>
        </div>
        {projectDialog === "backup" && backupStatus && (
          <p role="status" className="package-preview">
            {backupStatus}
          </p>
        )}
      </Dialog>

      <Dialog
        open={!!pendingCloseIntent || !!closeError}
        title="Before you leave"
        onClose={() => {
          setPendingCloseIntent(null);
          setCloseError(null);
        }}
      >
        {pendingCloseIntent && (
          <div role="alert" className="error-banner">
            <p>{closeWarningMessage(pendingCloseIntent)}</p>
            {entryControllerRef.current?.canSubmit !== false && (
              <button
                disabled={combinedSaveState === "saving"}
                onClick={() => void waitForSaveThenClose(pendingCloseIntent)}
              >
                Save and close
              </button>
            )}
            <button disabled={combinedSaveState === "saving"} onClick={handleForceCloseDiscarding}>
              {closeWarningActionLabel(pendingCloseIntent)}
            </button>
            <button onClick={() => setPendingCloseIntent(null)}>Cancel</button>
          </div>
        )}
        {closeError && (
          <p role="alert" className="error-banner">
            {closeError}
          </p>
        )}
      </Dialog>
    </main>
  );
}

function App() {
  const [project, setProject] = useState<ProjectSummary | null>(null);

  if (!project) {
    return <HomeScreen onOpened={setProject} />;
  }
  return <ProjectScreen project={project} onClosed={() => setProject(null)} />;
}

export default App;
