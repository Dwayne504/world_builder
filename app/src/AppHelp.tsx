import { useEffect, useState } from "react";
import { getBuildInfo, type BuildInfo } from "./api";
import { useAppearance } from "./appearanceContext";
import { useDesktopCommands } from "./desktopMenuContext";
import { Dialog } from "./Dialog";

/** Local reference: available offline and independent of any Project. */
export function AppHelp() {
  const appearance = useAppearance();
  const [page, setPage] = useState<"guide" | "about" | null>(null);
  const [build, setBuild] = useState<BuildInfo | null>(null);
  const [buildError, setBuildError] = useState(false);
  useEffect(() => {
    if (page !== "about" || !("__TAURI_INTERNALS__" in window)) return;
    let current = true;
    setBuildError(false);
    void getBuildInfo()
      .then((info) => {
        if (current) setBuild(info);
      })
      .catch(() => {
        if (current) setBuildError(true);
      });
    return () => {
      current = false;
    };
  }, [page]);
  useDesktopCommands(
    "app",
    {
      view: [
        { id: "appearance", label: "Appearance…", separatorBefore: true, action: appearance.open },
      ],
      help: [
        { id: "guide", label: "Using Worldcrafter", action: () => setPage("guide") },
        {
          id: "about",
          label: "About Worldcrafter",
          separatorBefore: true,
          action: () => setPage("about"),
        },
      ],
    },
    100,
  );
  return (
    <>
      <Dialog open={page === "guide"} title="Using Worldcrafter" onClose={() => setPage(null)}>
        <p>Start with an Entry, a Chapter, or a small idea. Add structure as it becomes useful.</p>
        <dl className="app-help-reference">
          <dt>File</dt>
          <dd>
            Create or open a Project from Home. In a Project, find backups, Project settings, and
            Close Project.
          </dd>
          <dt>Edit</dt>
          <dd>
            Manage the Entry, Category, Type, Fields, Relationships, Chapter, or Timeline you are
            working with. Open an Entry, Chapter, or Timeline to enable its commands.
          </dd>
          <dt>View</dt>
          <dd>
            Move between your world and your writing, show the sidebar, or change the app’s
            appearance.
          </dd>
          <dt>Quick creation</dt>
          <dd>
            Use Add Entry, Add field, Add relationship, New Chapter, or Add occurrence beside your
            work. You can create missing Categories and Types while adding an Entry.
          </dd>
          <dt>Saving</dt>
          <dd>
            The Project status shows whether changes are saved. Finish or cancel unfinished forms
            before leaving; Worldcrafter asks when a draft needs attention.
          </dd>
          <dt>Keyboard</dt>
          <dd>
            Tab to the menu bar. Use the arrow keys to move through menus, Enter to choose, and
            Escape to close. Alt+Left and Alt+Right move through workspace history.
          </dd>
        </dl>
      </Dialog>
      <Dialog open={page === "about"} title="About Worldcrafter" onClose={() => setPage(null)}>
        <p>A local workspace for your worlds and stories.</p>
        {!("__TAURI_INTERNALS__" in window) ? (
          <p className="muted">
            Browser preview. Desktop build information is available in the installed app.
          </p>
        ) : build ? (
          <dl>
            <dt>App version</dt>
            <dd>{build.version}</dd>
            <dt>Supported Project schema</dt>
            <dd>{build.supportedSchemaVersion}</dd>
            <dt>Supported package format</dt>
            <dd>{build.supportedFormatVersion}</dd>
          </dl>
        ) : (
          <p role="status">
            {buildError
              ? "Build information is unavailable in this app build."
              : "Loading build information…"}
          </p>
        )}
      </Dialog>
    </>
  );
}
