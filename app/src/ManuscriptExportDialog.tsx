import { useEffect, useRef, useState } from "react";
import {
  chooseManuscriptExportDestination,
  discardManuscriptExport,
  previewManuscriptExport,
  publishManuscriptExport,
  readStory,
} from "./api";
import { ChapterPagination } from "./ChapterPagination";
import { Dialog } from "./Dialog";
import { chapterLabel, type ChapterSummary } from "./storyTypes";
import type {
  ManuscriptExportDestination,
  ManuscriptExportPreview,
  ManuscriptExportReceipt,
} from "./manuscriptExportTypes";
import "./ManuscriptExportDialog.css";

const PAGE_SIZE = 20;
type Phase = "loading" | "previewing" | "choosing" | "publishing" | null;

/** Mount only while open: cancellation releases the backend-owned preview. */
export function ManuscriptExportDialog({
  projectId,
  beforePreview,
  onOperation,
  onClose,
}: {
  projectId: string;
  beforePreview: () => Promise<boolean>;
  onOperation: (operation: Promise<void>) => void;
  onClose: () => void;
}) {
  const [chapters, setChapters] = useState<ChapterSummary[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [view, setView] = useState("active");
  const [page, setPage] = useState(0);
  const [previewPage, setPreviewPage] = useState(0);
  const [preview, setPreview] = useState<ManuscriptExportPreview | null>(null);
  const [destination, setDestination] = useState<ManuscriptExportDestination | null>(null);
  const [receipt, setReceipt] = useState<ManuscriptExportReceipt | null>(null);
  const [phase, setPhase] = useState<Phase>("loading");
  const [error, setError] = useState<string | null>(null);
  const live = useRef(true);
  const generation = useRef(0);
  const pending = useRef<Promise<void> | null>(null);
  const previewToken = useRef<string | null>(null);
  const onOperationRef = useRef(onOperation);
  onOperationRef.current = onOperation;

  function releasePreview() {
    const token = previewToken.current;
    previewToken.current = null;
    if (token)
      void discardManuscriptExport(projectId, token).catch(() => {
        // Tokens are bounded, expire, and are invalidated by native Project close.
      });
  }
  const releaseRef = useRef(releasePreview);
  releaseRef.current = releasePreview;

  useEffect(() => {
    live.current = true;
    const request = ++generation.current;
    const task = readStory(projectId)
      .then((index) => {
        if (!live.current || generation.current !== request) return;
        setChapters([...index.chapters].sort((a, b) => a.readingRank - b.readingRank));
        setLoaded(true);
      })
      .catch((reason) => {
        if (live.current && generation.current === request)
          setError(reason instanceof Error ? reason.message : "Chapters could not be loaded.");
      })
      .finally(() => {
        if (live.current && generation.current === request) {
          pending.current = null;
          setPhase(null);
        }
      });
    pending.current = task;
    onOperationRef.current(task);
    return () => {
      live.current = false;
      releaseRef.current();
    };
  }, [projectId]);

  function run(nextPhase: Phase, operation: (request: number) => Promise<void>) {
    if (pending.current) return;
    setPhase(nextPhase);
    setError(null);
    const request = ++generation.current;
    const task = operation(request)
      .catch((reason) => {
        if (live.current && generation.current === request)
          setError(reason instanceof Error ? reason.message : "Export could not be completed.");
      })
      .finally(() => {
        if (pending.current === task) pending.current = null;
        if (live.current && generation.current === request) setPhase(null);
      });
    pending.current = task;
    onOperationRef.current(task);
  }
  const current = (request: number) => live.current && generation.current === request;

  function changeSelection(next: string[]) {
    if (pending.current) return;
    releasePreview();
    setPreview(null);
    setDestination(null);
    setReceipt(null);
    setError(null);
    setSelected(next);
  }
  function makePreview() {
    run("previewing", async (request) => {
      if (!(await beforePreview())) {
        throw new Error(
          "Save or discard unfinished edits in the editor before previewing the export.",
        );
      }
      if (!current(request)) return;
      releasePreview();
      setPreview(null);
      setDestination(null);
      setReceipt(null);
      const next = await previewManuscriptExport(projectId, selected);
      if (!current(request)) {
        await discardManuscriptExport(projectId, next.previewId).catch(() => {});
        return;
      }
      previewToken.current = next.previewId;
      setPreview(next);
      setPreviewPage(0);
    });
  }
  function chooseDestination() {
    if (!preview) return;
    run("choosing", async (request) => {
      setDestination(null);
      try {
        const chosen = await chooseManuscriptExportDestination(projectId, preview.previewId);
        if (current(request)) setDestination(chosen);
      } catch (reason) {
        releasePreview();
        if (current(request)) setPreview(null);
        throw new Error(
          `${reason instanceof Error ? reason.message : "The export file could not be chosen."} Create a new preview, then choose a file again.`,
        );
      }
    });
  }
  function publish() {
    if (!preview || !destination) return;
    run("publishing", async (request) => {
      try {
        const result = await publishManuscriptExport(
          projectId,
          preview.previewId,
          destination.destinationId,
          destination.replacesExisting,
        );
        if (current(request)) setReceipt(result);
      } catch (reason) {
        throw new Error(
          `${reason instanceof Error ? reason.message : "Export failed."} Create a new preview, then choose a file again.`,
        );
      } finally {
        releasePreview();
        if (current(request)) {
          setPreview(null);
          setDestination(null);
        }
      }
    });
  }
  function cancel() {
    if (phase === "publishing") return;
    live.current = false;
    ++generation.current;
    releasePreview();
    onClose();
  }

  const matches = chapters.filter(
    (chapter) =>
      (view === "selected" ? selected.includes(chapter.id) : chapter.workspaceState === view) &&
      chapterLabel(chapter).toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matches.length / PAGE_SIZE) - 1));
  const inactiveSelected = chapters.filter(
    (c) => selected.includes(c.id) && c.workspaceState !== "active",
  ).length;
  const busy = phase !== null;

  return (
    <Dialog open title="Export manuscript" onClose={cancel} className="manuscript-export-dialog">
      <p className="muted">
        Export selected Chapter manuscripts as Markdown, in reading order. Plan, Notes and Summary
        stay in Worldcrafter. Use Backups to preserve the whole Project.
      </p>
      {error && <p role="alert">{error}</p>}
      {receipt && (
        <div role="status">
          <p>
            Exported {receipt.chapterCount} Chapters · {receipt.wordCount} words.
          </p>
          <p className="package-preview">{receipt.path}</p>
        </div>
      )}
      {phase && (
        <p role="status">
          {phase === "loading"
            ? "Loading Chapters…"
            : phase === "previewing"
              ? "Saving writing and preparing preview…"
              : phase === "choosing"
                ? "Choose a file in Save As…"
                : "Exporting manuscript…"}
        </p>
      )}
      {!loaded && !busy && (
        <button
          onClick={() =>
            run("loading", async (request) => {
              const index = await readStory(projectId);
              if (!current(request)) return;
              setChapters([...index.chapters].sort((a, b) => a.readingRank - b.readingRank));
              setLoaded(true);
            })
          }
        >
          Retry loading Chapters
        </button>
      )}
      {loaded && (
        <>
          <fieldset disabled={busy} className="export-selection">
            <legend>
              Choose Chapters <small>{selected.length} selected</small>
            </legend>
            <div className="export-filters">
              <label>
                Find Chapters
                <input
                  type="search"
                  value={query}
                  onChange={(event) => {
                    setQuery(event.target.value);
                    setPage(0);
                  }}
                />
              </label>
              <label>
                Chapter list
                <select
                  value={view}
                  onChange={(event) => {
                    setView(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="active">Active Chapters</option>
                  <option value="archived">Archived Chapters</option>
                  <option value="trashed">Trash</option>
                  <option value="selected">Selected Chapters</option>
                </select>
              </label>
            </div>
            <div className="row">
              <button
                className="quiet-button"
                onClick={() =>
                  changeSelection([
                    ...new Set([
                      ...selected,
                      ...chapters.filter((c) => c.workspaceState === "active").map((c) => c.id),
                    ]),
                  ])
                }
              >
                Select all active Chapters
              </button>
              <button
                className="quiet-button"
                disabled={!selected.length}
                onClick={() => changeSelection([])}
              >
                Clear selection
              </button>
            </div>
            <ul className="export-chapter-list" aria-label="Chapters to export">
              {matches
                .slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE)
                .map((chapter) => (
                  <li key={chapter.id}>
                    <label>
                      <input
                        type="checkbox"
                        checked={selected.includes(chapter.id)}
                        onChange={(event) =>
                          changeSelection(
                            event.target.checked
                              ? [...selected, chapter.id]
                              : selected.filter((id) => id !== chapter.id),
                          )
                        }
                      />
                      <span>
                        {chapterLabel(chapter)}
                        <small>
                          {chapter.wordCount} words
                          {chapter.workspaceState !== "active"
                            ? ` · ${chapter.workspaceState}`
                            : ""}
                        </small>
                      </span>
                    </label>
                  </li>
                ))}
            </ul>
            <ChapterPagination
              label="export Chapters"
              total={matches.length}
              page={currentPage}
              pageSize={PAGE_SIZE}
              onPage={setPage}
            />
            {!!inactiveSelected && (
              <p className="field-note">
                Selection includes {inactiveSelected} archived or trashed Chapters.
              </p>
            )}
            <button disabled={!selected.length} onClick={makePreview}>
              {preview ? "Refresh preview" : "Preview export"}
            </button>
          </fieldset>
          {preview && (
            <section aria-label="Export preview" className="manuscript-export-preview">
              <h3>
                Preview · {preview.chapters.length} Chapters · {preview.wordCount} words
              </h3>
              <details>
                <summary>Included Chapters, in reading order</summary>
                <ol start={previewPage * PAGE_SIZE + 1} className="export-preview-order">
                  {preview.chapters
                    .slice(previewPage * PAGE_SIZE, (previewPage + 1) * PAGE_SIZE)
                    .map((chapter) => (
                      <li key={chapter.id}>
                        {chapter.title.trim() || "[Untitled Chapter]"}
                        <small>
                          {chapter.wordCount} words · {chapter.workspaceState}
                        </small>
                      </li>
                    ))}
                </ol>
                <ChapterPagination
                  label="preview Chapters"
                  page={previewPage}
                  pageSize={PAGE_SIZE}
                  total={preview.chapters.length}
                  onPage={setPreviewPage}
                />
              </details>
              {!!preview.formatNotes.length && (
                <ul className="field-note">
                  {preview.formatNotes.map((note) => (
                    <li key={note}>{note}</li>
                  ))}
                </ul>
              )}
              <label>
                Exact Markdown preview
                <textarea readOnly rows={10} value={preview.markdown} spellCheck={false} />
              </label>
              <p className="field-note">Suggested name: {preview.suggestedFileName}</p>
              <button disabled={busy} onClick={chooseDestination}>
                Choose export file…
              </button>
              {destination && (
                <div className="export-destination">
                  <p className="package-preview">{destination.path}</p>
                  <p>
                    {destination.replacesExisting
                      ? "This file already exists. Replace it with this manuscript?"
                      : "Save this manuscript to the selected file?"}
                  </p>
                  <button disabled={busy} onClick={publish}>
                    {destination.replacesExisting ? "Replace existing file" : "Export manuscript"}
                  </button>
                </div>
              )}
            </section>
          )}
        </>
      )}
      <div className="row">
        <button className="quiet-button" disabled={phase === "publishing"} onClick={cancel}>
          {receipt ? "Done" : "Cancel"}
        </button>
      </div>
    </Dialog>
  );
}
