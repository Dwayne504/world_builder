import { useCallback, useEffect, useRef, useState } from "react";
import { applyStory, readStory } from "./api";
import {
  chapterLabel,
  type ChapterController,
  type ChapterSnapshot,
  type StoryIndex,
  type StoryCommand,
  type WorkspaceState,
} from "./storyTypes";
import type { SubmitOutcome } from "./useProjectRename";

export function ChapterLibrary({
  projectId,
  onOpen,
  onController,
  onRevision,
}: {
  projectId: string;
  onOpen: (id: string, snapshot?: ChapterSnapshot) => void;
  onController: (controller: ChapterController) => void;
  onRevision: (revision: number) => void;
}) {
  const [index, setIndex] = useState<StoryIndex | null>(null);
  const [view, setView] = useState<WorkspaceState>("active");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const submit = useCallback(
    () => pending.current ?? Promise.resolve({ kind: "no-op" } as SubmitOutcome),
    [],
  );
  useEffect(
    () =>
      onController({ state: busy ? "saving" : "saved", canSubmit: true, submit, autoFlush: true }),
    [busy, onController, submit],
  );
  const reload = useCallback(async () => {
    try {
      setIndex(await readStory(projectId));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chapters could not be loaded.");
    }
  }, [projectId]);
  useEffect(() => {
    void reload();
  }, [reload]);
  function perform(command: StoryCommand, open = false) {
    if (!index || pending.current) return;
    setBusy(true);
    setError(null);
    const task = applyStory(projectId, index.globalRevision, command)
      .then(async (next): Promise<SubmitOutcome> => {
        onRevision(next.globalRevision);
        if (open) onOpen(next.chapter.id, next);
        else await reload();
        return { kind: "committed" };
      })
      .catch((e: unknown): SubmitOutcome => {
        setError(e instanceof Error ? e.message : "Chapter change failed.");
        return { kind: "failed" };
      })
      .finally(() => {
        pending.current = null;
        setBusy(false);
      });
    pending.current = task;
  }
  const chapters = index?.chapters.filter((c) => c.workspaceState === view) ?? [];
  return (
    <section className="panel chapter-library" aria-label="Chapters">
      <div className="section-heading">
        <div>
          <p className="eyebrow">YOUR STORY</p>
          <h2>Chapters</h2>
        </div>
        <button
          disabled={!index || busy}
          onClick={() => perform({ kind: "create", title: "" }, true)}
        >
          New Chapter
        </button>
      </div>
      <p className="muted">
        Start anywhere. Arrange your Chapters in reading order as your story grows.
      </p>
      <div className="row chapter-views" aria-label="Chapter view">
        {(["active", "archived", "trashed"] as const).map((state) => (
          <button
            key={state}
            className="quiet-button"
            disabled={busy}
            aria-pressed={view === state}
            onClick={() => setView(state)}
          >
            {state === "active" ? "Writing" : state === "archived" ? "Archive" : "Trash"}
          </button>
        ))}
      </div>
      {error && (
        <div role="alert">
          <p>{error}</p>
          <button disabled={busy} onClick={() => void reload()}>
            Reload Chapters
          </button>
        </div>
      )}
      {!index && !error && <p role="status">Loading Chapters…</p>}
      {index && !chapters.length && (
        <p>
          {view === "active"
            ? "Your first Chapter can be just a sentence, a title, or an idea."
            : "No Chapters here."}
        </p>
      )}
      <ol className="chapter-list">
        {chapters.map((chapter, i) => (
          <li key={chapter.id}>
            <div>
              <button
                className="chapter-link quiet-button"
                disabled={busy}
                onClick={() => onOpen(chapter.id)}
              >
                {chapterLabel(chapter)}
              </button>
              <small className="muted">{chapter.wordCount} words</small>
            </div>
            <div className="row">
              {view === "active" ? (
                <>
                  <button
                    className="quiet-button"
                    aria-label={`Move ${chapterLabel(chapter)} up`}
                    disabled={busy || i === 0}
                    onClick={() =>
                      perform({ kind: "move", chapterId: chapter.id, beforeId: chapters[i - 1].id })
                    }
                  >
                    ↑
                  </button>
                  <button
                    className="quiet-button"
                    aria-label={`Move ${chapterLabel(chapter)} down`}
                    disabled={busy || i === chapters.length - 1}
                    onClick={() =>
                      perform({
                        kind: "move",
                        chapterId: chapter.id,
                        beforeId: chapters[i + 2]?.id ?? null,
                      })
                    }
                  >
                    ↓
                  </button>
                </>
              ) : (
                <button
                  disabled={busy}
                  onClick={() =>
                    perform({ kind: "set_state", chapterId: chapter.id, state: "active" })
                  }
                >
                  Restore
                </button>
              )}
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
