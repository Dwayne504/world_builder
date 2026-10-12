import { useEffect, useState } from "react";
import { exploreProject } from "./api";
import { ManagerSearchSelect } from "./ManagerSearchSelect";
import {
  initialExploreView,
  type ExploreEntry,
  type ExploreResults,
  type ExploreView,
} from "./exploreTypes";

const capabilities: Record<string, string> = { base: "Base", spatial: "Spatial", event: "Event" };
const entryContext = (entry: ExploreEntry) =>
  [entry.category, entry.typeName, entry.workspaceState === "active" ? null : entry.workspaceState]
    .filter(Boolean)
    .join(" · ");

function OtherEntryPicker({
  projectId,
  onChoose,
}: {
  projectId: string;
  onChoose: (entry: ExploreEntry) => void;
}) {
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<ExploreResults | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let current = true;
    setResult(null);
    setError(null);
    const timer = window.setTimeout(() => {
      void exploreProject(projectId, { ...initialExploreView, query, workspaceState: "all" })
        .then((value) => {
          if (current) setResult(value);
        })
        .catch((e: unknown) => {
          if (current) setError(String(e));
        });
    }, 150);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [projectId, query, attempt]);
  return (
    <div className="explore-picker">
      <label>
        Find the other Entry
        <input value={query} maxLength={200} onChange={(e) => setQuery(e.target.value)} />
      </label>
      {error && (
        <p role="alert">
          {error} <button onClick={() => setAttempt((n) => n + 1)}>Try again</button>
        </p>
      )}
      {!error && !result && <p role="status">Finding Entries…</p>}
      {result && (
        <>
          <p className="muted">
            {result.total > result.entries.length
              ? `First ${result.entries.length} of ${result.total}. Search to narrow the list.`
              : `${result.total} Entries`}
          </p>
          <ul className="explore-picker-list">
            {result.entries.map((entry) => (
              <li key={entry.id}>
                <button className="quiet-button" onClick={() => onChoose(entry)}>
                  {entry.name}
                  <small>{entryContext(entry)}</small>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

export function Explore({
  projectId,
  view,
  onViewChange,
  onOpen,
  refreshKey = 0,
}: {
  projectId: string;
  view: ExploreView;
  onViewChange: (view: ExploreView) => void;
  onOpen: (id: string) => void;
  refreshKey?: number;
}) {
  const [result, setResult] = useState<ExploreResults | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [choosingOther, setChoosingOther] = useState(false);
  const requestKey = JSON.stringify(view);
  useEffect(() => {
    let current = true;
    setLoading(true);
    setError(null);
    const timer = window.setTimeout(() => {
      void exploreProject(projectId, JSON.parse(requestKey) as ExploreView)
        .then((next) => {
          if (current) {
            setResult(next);
            setLoading(false);
          }
        })
        .catch((e: unknown) => {
          if (current) {
            setError(String(e));
            setLoading(false);
          }
        });
    }, 150);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [projectId, requestKey, attempt, refreshKey]);
  const update = (change: Partial<ExploreView>) => onViewChange({ ...view, ...change, page: 0 });
  const relationship = view.relationship;
  const definition = result?.definitions.find((d) => d.id === relationship?.definitionId);
  const selectedOther =
    result?.selectedOther && result.selectedOther.id === relationship?.otherEntryId
      ? result.selectedOther
      : null;
  const updateRelationship = (change: Partial<NonNullable<ExploreView["relationship"]>>) => {
    if (relationship) update({ relationship: { ...relationship, ...change } });
  };
  return (
    <section className="panel explore" aria-labelledby="explore-title">
      <div className="section-header">
        <div>
          <h2 id="explore-title">Explore</h2>
          <p className="muted">Find Entries that match every selected filter.</p>
        </div>
        <button
          className="quiet-button"
          onClick={() => {
            onViewChange({ ...initialExploreView });
            setChoosingOther(false);
          }}
        >
          Clear filters
        </button>
      </div>
      <div className="explore-filters">
        <label>
          Name or alias
          <input
            maxLength={200}
            value={view.query}
            onChange={(e) => update({ query: e.target.value })}
          />
        </label>
        <ManagerSearchSelect
          label="Category"
          emptyLabel="Any Category"
          value={view.categoryId ?? ""}
          choices={[
            ...(result?.categories.map((c) => ({ id: c.id, label: c.name })) ?? []),
            ...(view.categoryId && !result?.categories.some((c) => c.id === view.categoryId)
              ? [{ id: view.categoryId, label: "Unavailable Category" }]
              : []),
          ]}
          onChange={(id) => update({ categoryId: id || null })}
        />
        <ManagerSearchSelect
          label="Exact Type"
          emptyLabel="Any Type"
          value={view.typeId ?? ""}
          choices={[
            ...(result?.types.map((t) => ({
              id: t.id,
              label: `${t.name} · ${result.categories.find((c) => c.id === t.categoryId)?.name ?? "Unavailable Category"}`,
            })) ?? []),
            ...(view.typeId && !result?.types.some((t) => t.id === view.typeId)
              ? [{ id: view.typeId, label: "Unavailable Type" }]
              : []),
          ]}
          onChange={(id) => update({ typeId: id || null })}
        />
        <label>
          Capability
          <select
            value={view.capability ?? ""}
            onChange={(e) => update({ capability: e.target.value || null })}
          >
            <option value="">Any capability</option>
            {view.capability && !result?.capabilities.includes(view.capability) && (
              <option value={view.capability}>Unavailable capability</option>
            )}
            {result?.capabilities.map((c) => (
              <option key={c} value={c}>
                {capabilities[c] ?? c}
              </option>
            ))}
          </select>
        </label>
        <label>
          Entries to include
          <select
            value={view.workspaceState}
            onChange={(e) =>
              update({ workspaceState: e.target.value as ExploreView["workspaceState"] })
            }
          >
            <option value="active">Active</option>
            <option value="archived">Archived</option>
            <option value="trashed">Trash</option>
            <option value="all">All states</option>
          </select>
        </label>
      </div>
      <fieldset className="explore-relationship">
        <legend>Current relationship</legend>
        <div className="explore-filters">
          <ManagerSearchSelect
            label="Relationship"
            emptyLabel="Any relationship or none"
            value={relationship?.definitionId ?? ""}
            choices={[
              ...(result?.definitions.map((d) => ({
                id: d.id,
                label: `${d.name}${d.retired ? " · Retired" : ""}`,
                disabled: d.retired,
              })) ?? []),
              ...(relationship && !definition
                ? [{ id: relationship.definitionId, label: "Unavailable relationship" }]
                : []),
            ]}
            onChange={(id) => {
              update({
                relationship: id
                  ? {
                      definitionId: id,
                      perspective: "source",
                      otherEntryId: null,
                      includeContained: false,
                    }
                  : null,
              });
              setChoosingOther(false);
            }}
          />
          {relationship && (
            <>
              <label>
                Result Entry’s side
                <select
                  value={relationship.perspective}
                  onChange={(e) =>
                    updateRelationship({ perspective: e.target.value as "source" | "target" })
                  }
                  disabled={definition?.directed === false}
                >
                  <option value="source">{definition?.forwardLabel ?? "Source"}</option>
                  <option value="target">{definition?.inverseLabel ?? "Target"}</option>
                </select>
              </label>
              <div className="explore-other">
                <span>Other Entry</span>
                <strong>
                  {relationship.otherEntryId
                    ? (selectedOther?.name ??
                      (loading ? "Loading selected Entry…" : "Unavailable Entry"))
                    : "Any Entry"}
                </strong>
                {selectedOther && <small className="muted">{entryContext(selectedOther)}</small>}
                <div className="button-row">
                  <button
                    className="quiet-button"
                    aria-expanded={choosingOther}
                    onClick={() => setChoosingOther((v) => !v)}
                  >
                    {choosingOther ? "Cancel selection" : "Choose Entry…"}
                  </button>
                  {relationship.otherEntryId && (
                    <button
                      className="quiet-button"
                      onClick={() =>
                        updateRelationship({ otherEntryId: null, includeContained: false })
                      }
                    >
                      Clear Entry
                    </button>
                  )}
                </div>
              </div>
            </>
          )}
        </div>
        {relationship && (
          <>
            {choosingOther && (
              <OtherEntryPicker
                projectId={projectId}
                onChoose={(entry) => {
                  updateRelationship({ otherEntryId: entry.id, includeContained: false });
                  setChoosingOther(false);
                }}
              />
            )}
            {relationship.otherEntryId && (
              <label>
                Match the other Entry
                <select
                  value={relationship.includeContained ? "contained" : "exact"}
                  onChange={(e) =>
                    updateRelationship({ includeContained: e.target.value === "contained" })
                  }
                >
                  <option value="exact">Exactly this Entry</option>
                  <option value="contained" disabled={!selectedOther?.spatial}>
                    This place and anywhere within it
                  </option>
                </select>
              </label>
            )}
            <p className="muted">
              {definition?.directed === false
                ? "This relationship works in either direction. "
                : "The result uses the selected side’s wording. "}
              Only current, active relationships count. For locations, choose the relationship your
              world uses for location. “Within” includes the selected place and its nested places,
              even when a place is archived or in Trash.
            </p>
          </>
        )}
      </fieldset>
      {error && (
        <p role="alert">
          {error} <button onClick={() => setAttempt((n) => n + 1)}>Try again</button>
        </p>
      )}
      {loading && <p role="status">Exploring…</p>}
      {!loading && !error && result && (
        <>
          {result.issues.length > 0 ? (
            <div role="alert">
              {result.issues.map((issue) => (
                <p key={issue}>{issue}</p>
              ))}
            </div>
          ) : (
            <>
              <div className="explore-results-header">
                <p role="status">
                  {result.total} matching Entries
                  {result.total > 0
                    ? ` · Showing ${result.entries.length ? result.page * result.pageSize + 1 : 0}–${result.entries.length ? result.page * result.pageSize + result.entries.length : 0}`
                    : ""}
                </p>
                <label>
                  Per page
                  <select
                    value={view.pageSize}
                    onChange={(e) => update({ pageSize: Number(e.target.value) })}
                  >
                    {[10, 20, 50, 100].map((n) => (
                      <option key={n} value={n}>
                        {n}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <ul className="explore-results">
                {result.entries.map((entry) => (
                  <li key={entry.id}>
                    <button
                      className="quiet-button"
                      data-navigation-focus={`explore-${entry.id}`}
                      onClick={() => onOpen(entry.id)}
                    >
                      <strong>{entry.name}</strong>
                      <small>{entryContext(entry)}</small>
                    </button>
                    {entry.relationshipMatch && entry.relationshipMatch !== "current" && (
                      <small className="muted">
                        {entry.relationshipMatch === "direct"
                          ? "Direct match"
                          : "Via contained place"}
                      </small>
                    )}
                  </li>
                ))}
              </ul>
              {!result.entries.length && (
                <p>No Entries on this page. Adjust the filters or return to the first page.</p>
              )}
              <div className="button-row" aria-label="Explore pages">
                <button disabled={!view.page} onClick={() => onViewChange({ ...view, page: 0 })}>
                  First
                </button>
                <button
                  disabled={!view.page}
                  onClick={() => onViewChange({ ...view, page: view.page - 1 })}
                >
                  Previous
                </button>
                <span>Page {view.page + 1}</span>
                <button
                  disabled={(view.page + 1) * view.pageSize >= result.total}
                  onClick={() => onViewChange({ ...view, page: view.page + 1 })}
                >
                  Next
                </button>
              </div>
            </>
          )}
        </>
      )}
    </section>
  );
}
