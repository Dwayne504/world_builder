import { useEffect, useState } from "react";
import { previewTypeRetirement } from "./api";
import { ChapterPagination } from "./ChapterPagination";
import type { TypeRetirementPreview } from "./types";

const PAGE_SIZE = 10;
export function TypeRetirementReview({
  projectId,
  typeId,
  busy,
  onConfirm,
  onCancel,
}: {
  projectId: string;
  typeId: string;
  busy: boolean;
  onConfirm: (revision: number, retired: boolean) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [preview, setPreview] = useState<TypeRetirementPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [query, setQuery] = useState("");
  const [view, setView] = useState("entries");
  const [page, setPage] = useState(0);
  const [submitting, setSubmitting] = useState(false);
  useEffect(() => {
    let current = true;
    setPreview(null);
    setError(null);
    void previewTypeRetirement(projectId, typeId)
      .then((value) => {
        if (current) setPreview(value);
      })
      .catch((reason) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "This Type could not be reviewed.");
      });
    return () => {
      current = false;
    };
  }, [projectId, typeId, attempt]);
  const rows = !preview
    ? []
    : view === "entries"
      ? preview.assignedEntries.map((entry) => ({
          id: entry.id,
          label: entry.label,
          context: `${entry.typeName}${entry.typeId !== preview.typeId ? " · inherits through child Type" : ""} · ${entry.workspaceState}`,
        }))
      : view === "types"
        ? preview.descendantTypes.map((type) => ({
            id: type.id,
            label: type.name,
            context: type.retiredAt
              ? "Retired child Type"
              : "Active child Type · remains available",
          }))
        : preview.defaultFields.map((field) => ({
            id: field.id,
            label: field.name,
            context: "Default Field · preserved",
          }));
  const matching = rows.filter((row) =>
    `${row.label} ${row.context}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matching.length / PAGE_SIZE) - 1));
  const locked = busy || submitting;
  return (
    <>
      {error && <p role="alert">{error}</p>}
      {!preview && !error && <p role="status">Reviewing Type…</p>}
      {preview && (
        <>
          <p>
            {preview.retiredAt ? (
              <>
                Restore <strong>{preview.name}</strong> for new Entries and child Types?
              </>
            ) : (
              <>
                Retire <strong>{preview.name}</strong>? It will no longer be offered for new Entries
                or as a parent for new Types.
              </>
            )}
          </p>
          <p>
            Existing Entries keep their Type and values. Child Types and inherited Fields continue
            to work. You can {preview.retiredAt ? "retire" : "restore"} this Type later.
          </p>
          <p className="field-note">
            {preview.directEntryCount} assigned Entries · {preview.inheritedEntryCount} Entries in
            child Types · {preview.descendantTypes.length} child Types ·{" "}
            {preview.defaultFields.length} default Fields
          </p>
          <details>
            <summary>Review existing uses</summary>
            <div className="row">
              <label>
                Show
                <select
                  disabled={locked}
                  value={view}
                  onChange={(event) => {
                    setView(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="entries">Entries</option>
                  <option value="types">Child Types</option>
                  <option value="fields">Default Fields</option>
                </select>
              </label>
              <label>
                Find existing uses
                <input
                  type="search"
                  disabled={locked}
                  value={query}
                  onChange={(event) => {
                    setQuery(event.target.value);
                    setPage(0);
                  }}
                />
              </label>
            </div>
            <ul className="manager-types manager-bounded-list" aria-label="Existing Type uses">
              {matching.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE).map((row) => (
                <li key={row.id}>
                  <span>
                    {row.label}
                    <small> · {row.context}</small>
                  </span>
                </li>
              ))}
            </ul>
            {!matching.length && <p className="muted">No matching uses.</p>}
            <ChapterPagination
              label="existing Type uses"
              page={currentPage}
              pageSize={PAGE_SIZE}
              total={matching.length}
              onPage={setPage}
            />
          </details>
          <button
            disabled={locked}
            onClick={async () => {
              setSubmitting(true);
              try {
                if (!(await onConfirm(preview.globalRevision, !preview.retiredAt))) {
                  setPreview(null);
                  setError("The Type was not changed. Refresh the review before trying again.");
                }
              } catch (reason) {
                setPreview(null);
                setError(
                  `${reason instanceof Error ? reason.message : "The Type was not changed."} Refresh the review before trying again.`,
                );
              } finally {
                setSubmitting(false);
              }
            }}
          >
            {preview.retiredAt ? "Restore Type" : "Retire Type"}
          </button>
        </>
      )}
      <button className="quiet-button" disabled={locked} onClick={() => setAttempt((n) => n + 1)}>
        Refresh review
      </button>
      <button disabled={locked} onClick={onCancel}>
        Cancel
      </button>
    </>
  );
}
