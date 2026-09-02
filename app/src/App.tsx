import { useState } from "react";
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

function errorMessage(err: unknown): string {
  if (err instanceof AppCommandError) {
    return `${err.message} (${err.kind})`;
  }
  return err instanceof Error ? err.message : String(err);
}

function HomeScreen({ onOpened }: { onOpened: (project: ProjectSummary) => void }) {
  const [baseDir, setBaseDir] = useState("");
  const [newName, setNewName] = useState("");
  const [openPath, setOpenPath] = useState("");
  const [backupPath, setBackupPath] = useState("");
  const [restoreDestination, setRestoreDestination] = useState("");
  const [restoreName, setRestoreName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleCreate() {
    setBusy(true);
    setError(null);
    try {
      onOpened(await createProject(baseDir, newName));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleOpen() {
    setBusy(true);
    setError(null);
    try {
      onOpened(await openProject(openPath));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleRestore() {
    setBusy(true);
    setError(null);
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
        <p role="alert" className="error-banner">
          {error}
        </p>
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
            onChange={(e) => setOpenPath(e.currentTarget.value)}
            placeholder="/path/to/Tortuga.wcproj"
          />
        </label>
        <button disabled={busy || !openPath} onClick={handleOpen}>
          Open Project
        </button>
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
  const [backupDir, setBackupDir] = useState("");
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const [closeWarning, setCloseWarning] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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

  async function handleClose() {
    if (rename.hasUnsavedWork) {
      setCloseWarning("This Project has unsaved changes. Retry saving or discard before closing.");
      return;
    }
    setBusy(true);
    try {
      await closeProject(project.projectId);
      onClosed();
    } catch (err) {
      setCloseWarning(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  function handleForceCloseDiscarding() {
    setCloseWarning(null);
    closeProject(project.projectId)
      .then(onClosed)
      .catch((err) => setCloseWarning(errorMessage(err)));
  }

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
            onBlur={() => rename.submit()}
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
        {closeWarning && (
          <div role="alert" className="error-banner">
            <p>{closeWarning}</p>
            <button onClick={handleForceCloseDiscarding}>Close anyway (discard changes)</button>
          </div>
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
