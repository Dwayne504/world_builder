import { useEffect, useState } from "react";
import {
  AppCommandError,
  forgetRecentProject,
  listRecentProjects,
  openRecentProject,
  pickDirectory,
} from "./api";
import type { ProjectSummary, RecentProject } from "./types";

export function RecentProjects({
  busy,
  onBusy,
  onOpened,
}: {
  busy: boolean;
  onBusy: (busy: boolean) => void;
  onOpened: (summary: ProjectSummary) => void;
}) {
  const [projects, setProjects] = useState<RecentProject[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [recovery, setRecovery] = useState<{ id: string; path?: string } | null>(null);
  useEffect(() => {
    let current = true;
    void listRecentProjects()
      .then((items) => {
        if (current) setProjects(items);
      })
      .catch((reason: unknown) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "Recent Projects are unavailable.");
      });
    return () => {
      current = false;
    };
  }, []);

  async function open(id: string, path?: string, recover = false) {
    onBusy(true);
    setError(null);
    setRecovery(null);
    try {
      onOpened(await openRecentProject(id, path, recover));
    } catch (reason) {
      if (reason instanceof AppCommandError && reason.kind === "lock_recovery_required") {
        setError(
          "This Project has a leftover lock from its last session. No active owner was found; you can recover it now.",
        );
        setRecovery({ id, path });
      } else if (reason instanceof AppCommandError && reason.kind === "identity_mismatch") {
        setError(
          "That folder belongs to a different Project. Locate this Project's folder, or open the other Project using Browse.",
        );
      } else {
        setError(reason instanceof Error ? reason.message : "Could not open this Project.");
      }
    } finally {
      onBusy(false);
    }
  }

  async function locate(project: RecentProject) {
    onBusy(true);
    setError(null);
    setRecovery(null);
    try {
      const path = await pickDirectory();
      if (path) await open(project.projectId, path);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not choose a folder.");
    } finally {
      onBusy(false);
    }
  }

  async function forget(id: string) {
    onBusy(true);
    setError(null);
    setRecovery(null);
    try {
      await forgetRecentProject(id);
      setProjects((items) => items.filter((item) => item.projectId !== id));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not update Recent Projects.");
    } finally {
      onBusy(false);
    }
  }

  return (
    <div className="recent-projects" aria-label="Recent Projects">
      {projects.length > 0 && (
        <>
          <h3>Recent Projects</h3>
          <ul>
            {projects.map((project) => (
              <li key={project.projectId}>
                <div className="recent-project-line">
                  <button
                    className="recent-project-name"
                    disabled={busy}
                    onClick={() => void open(project.projectId)}
                  >
                    {project.workingName}
                  </button>
                  {!project.available && <span className="muted">Folder unavailable</span>}
                  <details>
                    <summary aria-label={`Options for ${project.workingName}`}>Options</summary>
                    <p className="package-preview">{project.packagePath}</p>
                    <button disabled={busy} onClick={() => void locate(project)}>
                      Locate folder…
                    </button>
                    <button disabled={busy} onClick={() => void forget(project.projectId)}>
                      Remove from recent list
                    </button>
                  </details>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
      {error && <p role="alert">{error}</p>}
      {recovery && (
        <button disabled={busy} onClick={() => void open(recovery.id, recovery.path, true)}>
          Recover lock and open Project
        </button>
      )}
    </div>
  );
}
