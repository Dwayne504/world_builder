import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import {
  AppCommandError,
  closeProject,
  createBackup,
  createProject,
  openProject,
  restoreBackupAsCopy,
} from "./api";
import type { ProjectSummary } from "./types";
import { useProjectRename } from "./useProjectRename";
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

function ProjectScreen({ project, onClosed }: { project: ProjectSummary; onClosed: () => void }) {
  const rename = useProjectRename(project);
  const { submit: renameSubmit } = rename;
  const [backupDir, setBackupDir] = useState("");
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const [pendingCloseIntent, setPendingCloseIntent] = useState<CloseIntent | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const approvedNativeClose = useRef(false);
  const saveStateRef = useRef(rename.saveState);
  const closeInFlight = useRef<Promise<void> | null>(null);
  const nativeWindowCloseRequested = useRef(false);
  saveStateRef.current = rename.saveState;

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
      // The save already in flight cannot be cancelled, so we never treat it
      // as discardable: wait for it to settle, then close only if the
      // explicit outcome confirms the currently displayed draft committed.
      // Reading `saveState` back out of React state here would race the
      // render that applies it, so branch on the promise's own result
      // instead -- this also correctly refuses to close when a newer draft
      // was typed while the older save was still in flight.
      return renameSubmit().then((outcome) => {
        if (outcome.kind === "committed" || outcome.kind === "no-op") {
          return closeAfterBackend(intent);
        }
        return undefined;
      });
    },
    [renameSubmit, closeAfterBackend],
  );

  const requestClose = useCallback(
    (intent: CloseIntent): void => {
      setCloseError(null);
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
    [closeAfterBackend, waitForSaveThenClose],
  );

  useEffect(() => {
    if (rename.saveState === "saved") {
      setPendingCloseIntent(null);
    }
  }, [rename.saveState]);

  function handleClose() {
    requestClose("project");
  }

  function handleForceCloseDiscarding() {
    if (!pendingCloseIntent) {
      return;
    }
    const intent = pendingCloseIntent;
    setPendingCloseIntent(null);
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
        requestClose("native-window");
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
  }, [requestClose]);

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
          <span data-testid="save-state" className={`save-state save-state-${rename.saveState}`}>
            {saveStateLabel(rename.saveState)}
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
            <button onClick={handleForceCloseDiscarding}>
              {closeWarningActionLabel(pendingCloseIntent)}
            </button>
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
