import { useEffect, useState } from "react";
import { readProjectRelationships } from "./api";
import type { RelationshipParticipant, RelationshipSnapshot } from "./types";
import { initialRelationshipView, type RelationshipView } from "./workspaceHistory";

/** A read-only view of canonical relationships; editing stays with the Entry editor. */
export function RelationshipsBrowser({
  projectId,
  view,
  onViewChange,
  onNavigate,
  recentEntryIds = [],
  onRememberEntry,
}: {
  recentEntryIds?: string[];
  onRememberEntry?: (id: string) => void;
  projectId: string;
  view: RelationshipView;
  onViewChange: (view: RelationshipView) => void;
  onNavigate: (id: string) => void;
}) {
  const [snapshot, setSnapshot] = useState<RelationshipSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [entrySearch, setEntrySearch] = useState("");
  const [choiceLimit, setChoiceLimit] = useState(10);
  useEffect(() => {
    let current = true;
    setSnapshot(null);
    setError(null);
    void readProjectRelationships(projectId)
      .then((data) => {
        if (current) setSnapshot(data);
      })
      .catch((reason) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "Relationships could not be loaded.");
      });
    return () => {
      current = false;
    };
  }, [projectId, attempt]);

  function filter(change: Partial<RelationshipView>) {
    onViewChange({ ...view, relationshipId: undefined, ...change, visibleCount: 5 });
  }
  function participant(person: RelationshipParticipant, focusKey: string) {
    return person.id && person.workspaceState === "active" ? (
      <button
        className="relationship-entry-link quiet-button"
        data-navigation-focus={focusKey}
        onClick={() => onNavigate(person.id!)}
      >
        {person.label}
      </button>
    ) : (
      <span className="relationship-unavailable">
        {person.label} <small>({person.workspaceState})</small>
      </span>
    );
  }

  const participants = new Map(
    snapshot?.entries.map((entry) => [
      entry.id,
      { label: entry.label, detail: entry.categoryName },
    ]),
  );
  for (const relation of snapshot?.relationships ?? [])
    for (const person of [relation.source, relation.target])
      if (person.id && !participants.has(person.id))
        participants.set(person.id, { label: person.label, detail: person.workspaceState });
  const choices = [...participants].sort(
    (a, b) => a[1].label.localeCompare(b[1].label) || a[0].localeCompare(b[0]),
  );
  const query = entrySearch.trim().toLocaleLowerCase();
  const searchResults = choices.filter(([, entry]) =>
    `${entry.label} ${entry.detail}`.toLocaleLowerCase().includes(query),
  );
  const visibleChoices = query
    ? searchResults.slice(0, choiceLimit)
    : recentEntryIds.slice(0, 8).flatMap((id) => {
        const entry = participants.get(id);
        return entry ? [[id, entry] as const] : [];
      });
  const matching = (snapshot?.relationships ?? []).filter(
    (relation) =>
      (!view.relationshipId || relation.id === view.relationshipId) &&
      (!view.definitionId || relation.definitionId === view.definitionId) &&
      (!view.entryIds.length ||
        view.entryIds.some((id) => relation.source.id === id || relation.target.id === id)) &&
      (view.state === "all" ||
        (view.state === "ended"
          ? relation.ended
          : !relation.ended && relation.workspaceState === "active")),
  );
  const filtered = !!(
    view.relationshipId ||
    view.entryIds.length ||
    view.definitionId ||
    view.state !== "all"
  );

  return (
    <section className="relationships-browser" aria-labelledby="relationships-title">
      <p className="eyebrow">CONNECTIONS</p>
      <h2 id="relationships-title">Relationships</h2>
      <p className="muted">
        Explore your world’s connections. Open an Entry to edit its relationships.
      </p>
      {error ? (
        <div role="alert">
          <p>{error}</p>
          <button onClick={() => setAttempt((value) => value + 1)}>
            Retry loading relationships
          </button>
        </div>
      ) : !snapshot ? (
        <p role="status">Loading relationships…</p>
      ) : (
        <>
          {view.relationshipId && (
            <p className="field-note">
              Showing the connection you found in Search. Clear filters to explore all
              relationships.
            </p>
          )}
          <div className="relationship-filters">
            <div className="relationship-entry-filter">
              <label>
                Find an Entry
                <input
                  type="search"
                  value={entrySearch}
                  onChange={(event) => {
                    setEntrySearch(event.currentTarget.value);
                    setChoiceLimit(10);
                  }}
                />
              </label>
              <p className="field-note">
                Include either side of a connection. Selected Entries appear first.
              </p>
              {view.entryIds.length > 0 && (
                <div className="relationship-selected" aria-label="Selected Entries">
                  {view.entryIds.map((id) => (
                    <button
                      key={id}
                      className="quiet-button"
                      aria-label={`Remove ${participants.get(id)?.label ?? "Entry"} filter`}
                      onClick={() =>
                        filter({ entryIds: view.entryIds.filter((chosen) => chosen !== id) })
                      }
                    >
                      {participants.get(id)?.label ?? "Unavailable Entry"}{" "}
                      <span aria-hidden="true">×</span>
                    </button>
                  ))}
                </div>
              )}
              <p className="field-note">{query ? "Search results" : "Recent Entries"}</p>
              <div className="relationship-entry-choices">
                {visibleChoices.map(([id, entry]) => (
                  <label className="relationship-entry-choice" key={id}>
                    <input
                      type="checkbox"
                      checked={view.entryIds.includes(id)}
                      onChange={(event) => {
                        if (event.currentTarget.checked) onRememberEntry?.(id);
                        filter({
                          entryIds: event.currentTarget.checked
                            ? [...view.entryIds, id]
                            : view.entryIds.filter((chosen) => chosen !== id),
                        });
                      }}
                    />
                    <span>
                      {entry.label} <small className="muted">· {entry.detail}</small>
                    </span>
                  </label>
                ))}
              </div>
              {!visibleChoices.length && (
                <p className="field-note">
                  {query ? "No Entries match this search." : "Search for an Entry to get started."}
                </p>
              )}
              {query && searchResults.length > choiceLimit && (
                <button
                  className="quiet-button"
                  onClick={() => setChoiceLimit((count) => count + 10)}
                >
                  Show more Entry results
                </button>
              )}
              {view.entryIds.length > 0 && (
                <button className="quiet-button" onClick={() => filter({ entryIds: [] })}>
                  All Entries
                </button>
              )}
            </div>
            <label>
              Relationship
              <select
                aria-label="Relationship"
                value={view.definitionId}
                onChange={(event) => filter({ definitionId: event.currentTarget.value })}
              >
                <option value="">All relationships</option>
                {snapshot.definitions.map((definition) => (
                  <option key={definition.id} value={definition.id}>
                    {definition.name} · {definition.forwardLabel}
                    {definition.retired ? " (retired)" : ""}
                  </option>
                ))}
              </select>
            </label>
            <label>
              State
              <select
                aria-label="State"
                value={view.state}
                onChange={(event) =>
                  filter({ state: event.currentTarget.value as RelationshipView["state"] })
                }
              >
                <option value="all">Current and past</option>
                <option value="current">Current</option>
                <option value="ended">Ended</option>
              </select>
            </label>
            {filtered && (
              <button
                className="quiet-button"
                onClick={() => {
                  setEntrySearch("");
                  onViewChange(initialRelationshipView);
                }}
              >
                Clear filters
              </button>
            )}
          </div>
          <p className="muted relationship-result-count" role="status">
            Showing {Math.min(view.visibleCount, matching.length)} of {matching.length}{" "}
            {filtered ? "matching " : ""}relationships
          </p>
          {matching.length === 0 && (
            <p className="empty-state">
              {snapshot.relationships.length
                ? "No relationships match these filters."
                : "No relationships yet. Open an Entry and choose Add relationship to make a connection."}
            </p>
          )}
          <div className="relationship-card-grid">
            {matching.slice(0, view.visibleCount).map((relation) => {
              const definition = snapshot.definitions.find(
                (item) => item.id === relation.definitionId,
              );
              // Change the reading perspective, never the canonical participants.
              const preferred = view.entryIds.find(
                (id) => id === relation.source.id || id === relation.target.id,
              );
              const reverse =
                preferred === relation.target.id && relation.source.id !== relation.target.id;
              const first = reverse ? relation.target : relation.source;
              const second = reverse ? relation.source : relation.target;
              const label =
                (reverse && definition?.directed
                  ? definition.inverseLabel
                  : definition?.forwardLabel) ?? "connects to";
              return (
                <article
                  className="relationship-card"
                  key={relation.id}
                  aria-label={`${first.label} ${label} ${second.label}`}
                >
                  <div className="relationship-card-heading">
                    <span>{definition?.name ?? "Relationship"}</span>
                    <small>
                      {relation.ended ? "Ended" : "Current"}
                      {relation.workspaceState !== "active" ? ` · ${relation.workspaceState}` : ""}
                      {definition?.retired ? " · Retired definition" : ""}
                    </small>
                  </div>
                  <div className="relationship-card-connection">
                    {participant(first, `${relation.id}:${reverse ? "target" : "source"}`)}
                    <span className="relationship-card-verb">
                      <span aria-hidden="true">{definition?.directed ? "→" : "↔"}</span> {label}
                    </span>
                    {participant(second, `${relation.id}:${reverse ? "source" : "target"}`)}
                  </div>
                  {relation.note && (
                    <details className="relationship-card-note">
                      <summary>Note</summary>
                      <p>{relation.note}</p>
                    </details>
                  )}
                  {relation.warnings.map((warning) => (
                    <p className="relationship-warning" key={warning}>
                      {warning}
                    </p>
                  ))}
                </article>
              );
            })}
          </div>
          {matching.length > view.visibleCount && (
            <button
              className="relationships-more"
              onClick={() => onViewChange({ ...view, visibleCount: view.visibleCount + 5 })}
            >
              Show more relationships
            </button>
          )}
        </>
      )}
    </section>
  );
}
