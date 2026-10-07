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
import { useDesktopCommands } from "./desktopMenuContext";
import { Dialog } from "./Dialog";
import { ChapterPagination } from "./ChapterPagination";
import "./ChapterWorkspace.css";

const PAGE_SIZE = 20;
type ChapterSort = "reading" | "title" | "words";
export interface ChapterBrowseState {
  view: WorkspaceState;
  query: string;
  sort: ChapterSort;
  page: number;
}

export function ChapterLibrary({
  projectId,
  onOpen,
  onController,
  onRevision,
  locked = false,
  initialBrowseState,
  onBrowseStateChange,
}: {
  projectId: string;
  onOpen: (id: string, snapshot?: ChapterSnapshot) => void;
  onController: (controller: ChapterController) => void;
  onRevision: (revision: number) => void;
  locked?: boolean;
  initialBrowseState?: ChapterBrowseState;
  onBrowseStateChange?: (state: ChapterBrowseState) => void;
}) {
  const [index, setIndex] = useState<StoryIndex | null>(null);
  const [view, setView] = useState<WorkspaceState>(initialBrowseState?.view ?? "active");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState(initialBrowseState?.query ?? "");
  const [sort, setSort] = useState<ChapterSort>(initialBrowseState?.sort ?? "reading");
  const [page, setPage] = useState(initialBrowseState?.page ?? 0);
  const [movingId, setMovingId] = useState<string | null>(null);
  const [movePosition, setMovePosition] = useState("");
  const searchInput = useRef<HTMLInputElement>(null);
  const returnToSearch = useRef(false);
  const browseChanged = useRef(onBrowseStateChange);
  browseChanged.current = onBrowseStateChange;
  useEffect(() => {
    browseChanged.current?.({ view, query, sort, page });
  }, [view, query, sort, page]);
  useEffect(() => {
    if (movingId || !returnToSearch.current) return;
    returnToSearch.current = false;
    // Let the native dialog close before returning to a control that survives reordering.
    const frame = requestAnimationFrame(() => searchInput.current?.focus({ preventScroll: true }));
    return () => cancelAnimationFrame(frame);
  }, [movingId]);
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
    if (!index || pending.current || locked) return;
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
    return task;
  }
  const ordered = [...(index?.chapters ?? [])].sort((a, b) => a.readingRank - b.readingRank);
  const writing = ordered.filter((chapter) => chapter.workspaceState === "active");
  const positions = new Map(writing.map((chapter, position) => [chapter.id, position + 1]));
  const chapters = ordered.filter((chapter) => chapter.workspaceState === view);
  const search = query.trim().toLocaleLowerCase();
  const matches = chapters.filter(
    (chapter) =>
      chapterLabel(chapter).toLocaleLowerCase().includes(search) ||
      (view === "active" && String(positions.get(chapter.id)) === search.replace(/^#/, "")),
  );
  if (sort === "title")
    matches.sort((a, b) =>
      chapterLabel(a).localeCompare(chapterLabel(b), undefined, { numeric: true }),
    );
  if (sort === "words") matches.sort((a, b) => b.wordCount - a.wordCount);
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matches.length / PAGE_SIZE) - 1));
  const visible = matches.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE);
  const moving = writing.find((chapter) => chapter.id === movingId);
  const destination = Number(movePosition);
  const validDestination =
    Number.isInteger(destination) && destination >= 1 && destination <= writing.length;
  const remaining = writing.filter((chapter) => chapter.id !== movingId);
  const destinationBefore = validDestination ? remaining[destination - 1] : undefined;
  useDesktopCommands(
    "chapter-library",
    {
      edit: [
        {
          id: "chapter",
          label: "Chapter",
          children: [
            {
              id: "chapter-new",
              label: "New Chapter",
              disabled: !index || busy || locked,
              action: () => perform({ kind: "create", title: "" }, true),
            },
          ],
        },
      ],
    },
    20,
  );
  return (
    <section className="panel chapter-library" aria-label="Chapters">
      <div className="section-heading">
        <div>
          <p className="eyebrow">YOUR STORY</p>
          <h2>Chapters</h2>
        </div>
        <button
          disabled={!index || busy || locked}
          onClick={() => perform({ kind: "create", title: "" }, true)}
        >
          New Chapter
        </button>
      </div>
      <p className="muted chapter-library-intro">Your manuscript, in reading order.</p>
      <div className="row chapter-views" aria-label="Chapter view">
        {(["active", "archived", "trashed"] as const).map((state) => (
          <button
            key={state}
            className="quiet-button"
            disabled={busy}
            aria-pressed={view === state}
            aria-label={state === "active" ? "Writing" : state === "archived" ? "Archive" : "Trash"}
            onClick={() => {
              setView(state);
              setPage(0);
            }}
          >
            {state === "active" ? "Writing" : state === "archived" ? "Archive" : "Trash"}
            <span className="chapter-view-count" aria-hidden="true">
              {ordered.filter((chapter) => chapter.workspaceState === state).length}
            </span>
          </button>
        ))}
      </div>
      <div className="chapter-library-toolbar">
        <label>
          Find a Chapter
          <input
            ref={searchInput}
            type="search"
            placeholder="Title or reading position…"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(0);
            }}
          />
        </label>
        <label>
          Display order
          <select
            value={sort}
            onChange={(event) => {
              setSort(event.target.value as ChapterSort);
              setPage(0);
            }}
          >
            <option value="reading">Reading order</option>
            <option value="title">Title A–Z</option>
            <option value="words">Word count</option>
          </select>
        </label>
      </div>
      <div className="chapter-library-summary">
        <small className="muted">
          {chapters.length.toLocaleString()} Chapters ·{" "}
          {chapters.reduce((total, chapter) => total + chapter.wordCount, 0).toLocaleString()} words
        </small>
        {(query || sort !== "reading") && (
          <small className="muted">
            Search and display order keep your reading order unchanged.
          </small>
        )}
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
      {index && !!chapters.length && !matches.length && (
        <p className="chapter-empty">
          No Chapters match “{query}”.{" "}
          <button
            className="quiet-button"
            onClick={() => {
              setQuery("");
              setPage(0);
            }}
          >
            Clear search
          </button>
        </p>
      )}
      <ol className="chapter-library-list" aria-label="Chapter results">
        {visible.map((chapter) => (
          <li key={chapter.id}>
            {view === "active" && (
              <span
                className="chapter-reading-number"
                aria-label={`Reading position ${positions.get(chapter.id)}`}
              >
                {String(positions.get(chapter.id)).padStart(2, "0")}
              </span>
            )}
            <div className="chapter-library-title">
              <button
                className="chapter-link quiet-button"
                title={chapterLabel(chapter)}
                disabled={busy || locked}
                onClick={() => onOpen(chapter.id)}
              >
                {chapterLabel(chapter)}
              </button>
              <small className="muted">{chapter.wordCount.toLocaleString()} words</small>
            </div>
            <div className="row">
              {view === "active" ? (
                <button
                  className="quiet-button"
                  aria-label={`Move ${chapterLabel(chapter)} in reading order`}
                  disabled={busy || locked || writing.length < 2}
                  onClick={() => {
                    setMovingId(chapter.id);
                    setMovePosition(String(positions.get(chapter.id)));
                  }}
                >
                  Move…
                </button>
              ) : (
                <button
                  disabled={busy || locked}
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
      {index && matches.length > 0 && (
        <ChapterPagination
          label="Chapter results"
          page={currentPage}
          pageSize={PAGE_SIZE}
          total={matches.length}
          onPage={setPage}
          disabled={busy}
        />
      )}
      <Dialog
        open={!!moving}
        title="Move Chapter"
        onClose={() => {
          if (!busy) setMovingId(null);
        }}
      >
        {moving && (
          <>
            <p>
              <strong>{chapterLabel(moving)}</strong> is at position {positions.get(moving.id)} of{" "}
              {writing.length} in Writing.
            </p>
            <label>
              New reading position
              <input
                type="number"
                min={1}
                max={writing.length}
                step={1}
                value={movePosition}
                disabled={busy || locked}
                onChange={(event) => setMovePosition(event.target.value)}
              />
            </label>
            <p className="muted">
              This changes the manuscript’s reading order across all Writing Chapters, including
              those hidden by search.
            </p>
            {validDestination ? (
              <p>
                {destination === positions.get(moving.id)
                  ? "Already at this position."
                  : destinationBefore
                    ? `Place before ${chapterLabel(destinationBefore)}.`
                    : "Place at the end of Writing."}
              </p>
            ) : (
              <p role="status">Choose a position from 1 to {writing.length}.</p>
            )}
            {error && <p role="alert">{error}</p>}
            <div className="row">
              <button
                disabled={
                  busy || locked || !validDestination || destination === positions.get(moving.id)
                }
                onClick={() => {
                  void perform({
                    kind: "move",
                    chapterId: moving.id,
                    beforeId: destinationBefore?.id ?? null,
                  })?.then((result) => {
                    if (result.kind === "committed") {
                      returnToSearch.current = true;
                      setMovingId(null);
                    }
                  });
                }}
              >
                {busy ? "Moving…" : "Move Chapter"}
              </button>
              <button className="quiet-button" disabled={busy} onClick={() => setMovingId(null)}>
                Cancel
              </button>
            </div>
          </>
        )}
      </Dialog>
    </section>
  );
}
