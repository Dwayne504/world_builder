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
  listCategories,
  listEntries,
  listTypes,
  openProject,
  restoreBackupAsCopy,
} from "./api";
import type { Category, Entry, ProjectSummary, SaveState, TypeDef } from "./types";
import { useProjectRename } from "./useProjectRename";
import type { SubmitOutcome } from "./useProjectRename";
import { useEntryName } from "./useEntryName";
import { useMutationCoordinator } from "./useMutationCoordinator";
import { decideClose, type CloseIntent } from "./closeDecision";

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
          "power loss), so a leftover lock is still recorded. Because that record is old, " +
          "you can recover the Project and open it."
        );
      case "lock_held":
        return (
          "This Project is currently open in another Worldcrafter instance. Close it there " +
          "first; an active Project is never taken over."
        );
      case "lock_not_stale":
        return (
          "This Project may still be in use: it was closed only very recently, or another " +
          "instance may still be running. If another Worldcrafter is open, close it and try " +
          "again. Otherwise wait a while before trying again."
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

function HomeScreen({ onOpened }: { onOpened: (project: ProjectSummary) => void }) {
  const [baseDir, setBaseDir] = useState("");
  const [newName, setNewName] = useState("");
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

  async function handleCreate() {
    setBusy(true);
    setError(null);
    setErrorDetails(null);
    try {
      onOpened(await createProject(baseDir, newName));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
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

  return (
    <main className="container">
      <h1>Worldcrafter</h1>
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

      <section>
        <h2>New Project</h2>
        <label>
          Location
          <input
            aria-label="new-project-location"
            value={baseDir}
            onChange={(e) => setBaseDir(e.currentTarget.value)}
            placeholder="/path/to/projects"
          />
        </label>
        <label>
          Working name
          <input
            aria-label="new-project-name"
            value={newName}
            onChange={(e) => setNewName(e.currentTarget.value)}
            placeholder="Tortuga"
          />
        </label>
        <button disabled={busy || !baseDir || !newName} onClick={handleCreate}>
          Create Project
        </button>
      </section>

      <section>
        <h2>Open Project</h2>
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
            placeholder="/path/to/Tortuga.wcproj"
          />
        </label>
        <button disabled={busy || !openPath} onClick={handleOpen}>
          Open Project
        </button>
        {recoveryPath !== null && recoveryPath === openPath && (
          <div role="alert" className="error-banner">
            <p>
              Recovering removes only the leftover lock record; the Project's content is not
              modified. Use this only when you are sure no other Worldcrafter instance currently has
              this Project open.
            </p>
            <button disabled={busy} onClick={handleRecoverLock}>
              Recover lock and open Project
            </button>
          </div>
        )}
      </section>

      <section>
        <h2>Restore Backup as Copy</h2>
        <label>
          Backup path
          <input
            aria-label="restore-backup-path"
            value={backupPath}
            onChange={(e) => setBackupPath(e.currentTarget.value)}
            placeholder="/path/to/backup.wcbackup"
          />
        </label>
        <label>
          Destination folder
          <input
            aria-label="restore-destination"
            value={restoreDestination}
            onChange={(e) => setRestoreDestination(e.currentTarget.value)}
          />
        </label>
        <label>
          New working name (optional)
          <input
            aria-label="restore-new-name"
            value={restoreName}
            onChange={(e) => setRestoreName(e.currentTarget.value)}
          />
        </label>
        <button disabled={busy || !backupPath || !restoreDestination} onClick={handleRestore}>
          Restore as Copy
        </button>
      </section>
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
  onController,
  mutations,
}: {
  projectId: string;
  initialEntry: Entry;
  categories: Category[];
  onChanged: (entry: Entry) => void;
  onClose: () => void;
  onController: (controller: EntrySaveController) => void;
  mutations: MutationCoordinator;
}) {
  const editor = useEntryName(projectId, initialEntry);
  const { submit } = editor;
  const [types, setTypes] = useState<TypeDef[]>([]);
  const [categoryId, setCategoryId] = useState(editor.entry.categoryId);
  const [typeId, setTypeId] = useState(editor.entry.typeId ?? "");
  const [structureError, setStructureError] = useState<string | null>(null);
  const [structureTypeChosen, setStructureTypeChosen] = useState(true);
  const onChangedRef = useRef(onChanged);
  onChangedRef.current = onChanged;
  const structureDirty =
    categoryId !== editor.entry.categoryId || typeId !== (editor.entry.typeId ?? "");
  const combinedEntryState: SaveState =
    editor.saveState === "saving" || mutations.state === "saving"
      ? "saving"
      : editor.saveState === "failed" || mutations.state === "failed"
        ? "failed"
        : editor.saveState === "dirty" || structureDirty
          ? "dirty"
          : "saved";

  useEffect(() => {
    onController({ state: combinedEntryState, submit, canSubmit: !structureDirty });
  }, [combinedEntryState, onController, structureDirty, submit]);

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
        const nameOutcome = await submit();
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
    <section>
      <h2>{editor.entry.displayName}</h2>
      <p>Entry ID: {editor.entry.id}</p>
      <label>
        Name (optional)
        <input
          aria-label="entry-name"
          disabled={mutations.state === "saving"}
          value={editor.draftName}
          onChange={(event) => editor.onChangeDraft(event.currentTarget.value)}
        />
      </label>
      <span data-testid="entry-save-state">{saveStateLabel(combinedEntryState)}</span>
      {editor.errorMessage && <p role="alert">{editor.errorMessage}</p>}
      <label>
        Category
        <select
          aria-label="entry-category"
          disabled={mutations.state === "saving"}
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
          disabled={mutations.state === "saving"}
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
            !structureDirty ||
            (categoryId !== editor.entry.categoryId && !structureTypeChosen)
          }
          onClick={() => void saveStructure()}
        >
          Apply Category / Type
        </button>
        <button onClick={onClose}>Back to Entries</button>
      </div>
      {structureError && <p role="alert">{structureError}</p>}
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
          onClose={closeEditor}
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
    <section>
      <h2>Entries</h2>
      {error && <p role="alert">{error}</p>}
      <ul>
        {entries.map((entry) => (
          <li key={entry.id}>
            <button disabled={mutations.state === "saving"} onClick={() => openEntry(entry)}>
              {entry.displayName}
            </button>
          </li>
        ))}
      </ul>
      <h3>Create Entry</h3>
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
      <button disabled={mutations.state === "saving"} onClick={() => setShowCategoryCreator(true)}>
        Create Category inline
      </button>
      {showCategoryCreator && (
        <div>
          <input
            aria-label="inline-category-name"
            value={newCategoryName}
            onChange={(event) => setNewCategoryName(event.currentTarget.value)}
          />
          <button
            disabled={!newCategoryName.trim() || mutations.state === "saving"}
            onClick={() => void addCategory()}
          >
            Add Category
          </button>
        </div>
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
        <div>
          <input
            aria-label="inline-type-name"
            value={newTypeName}
            onChange={(event) => setNewTypeName(event.currentTarget.value)}
          />
          <button
            disabled={!newTypeName.trim() || mutations.state === "saving"}
            onClick={() => void addType()}
          >
            Add Type
          </button>
        </div>
      )}
      <button disabled={mutations.state === "saving"} onClick={() => void addEntry()}>
        Create Entry
      </button>
    </section>
  );
}

function ProjectScreen({ project, onClosed }: { project: ProjectSummary; onClosed: () => void }) {
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
          if (entryControllerRef.current?.canSubmit === false) {
            setPendingCloseIntent(waitingCloseIntentRef.current ?? intent);
            return undefined;
          }
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
    <main className="container">
      <h1>Worldcrafter</h1>

      <section>
        <h2>Project</h2>
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
          <span data-testid="save-state" className={`save-state save-state-${combinedSaveState}`}>
            {saveStateLabel(combinedSaveState)}
          </span>
        </div>
        {rename.errorMessage && (
          <p role="alert" className="error-banner">
            {rename.errorMessage}
          </p>
        )}

        <dl>
          <dt>Project ID</dt>
          <dd data-testid="project-id">{project.projectId}</dd>
          <dt>Location</dt>
          <dd>{project.packagePath}</dd>
        </dl>
      </section>

      <EntryWorkflow
        projectId={project.projectId}
        onController={receiveEntryController}
        onGlobalRevision={rename.updateRevision}
        mutations={mutations}
      />

      <section>
        <h2>Manual Backup</h2>
        <label>
          Backup destination folder
          <input
            aria-label="backup-destination"
            value={backupDir}
            onChange={(e) => setBackupDir(e.currentTarget.value)}
          />
        </label>
        <button disabled={busy || !backupDir} onClick={handleCreateBackup}>
          Create Manual Backup
        </button>
        {backupStatus && <p>{backupStatus}</p>}
      </section>

      <section>
        <h2>Close Project</h2>
        {pendingCloseIntent && (
          <div role="alert" className="error-banner">
            <p>{closeWarningMessage(pendingCloseIntent)}</p>
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
        <button disabled={busy} onClick={handleClose}>
          Close Project
        </button>
      </section>
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
