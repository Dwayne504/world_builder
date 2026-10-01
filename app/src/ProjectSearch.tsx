import { useEffect, useState } from "react";
import { searchProject } from "./api";
import type { SearchResults, SearchTarget, SearchView } from "./searchTypes";

const titles = {
  entries: "Entries",
  chapters: "Chapters",
  structured: "Fields & connections",
  text: "Manuscript & other text",
};

export function ProjectSearch({
  projectId,
  view,
  onViewChange,
  onOpen,
}: {
  projectId: string;
  view: SearchView;
  onViewChange: (view: SearchView) => void;
  onOpen: (target: SearchTarget) => void;
}) {
  const [result, setResult] = useState<SearchResults | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const { query, includeInactive, limitPerGroup } = view;
  useEffect(() => {
    let current = true;
    setResult(null);
    setError(null);
    if (!query.trim()) return;
    const timer = window.setTimeout(() => {
      void searchProject(projectId, { query, includeInactive, limitPerGroup })
        .then((next) => {
          if (current) setResult(next);
        })
        .catch((reason: unknown) => {
          if (current)
            setError(reason instanceof Error ? reason.message : "Search could not be loaded.");
        });
    }, 200);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [projectId, query, includeInactive, limitPerGroup, attempt]);
  const total = result?.groups.reduce((sum, group) => sum + group.total, 0) ?? 0;
  return (
    <section className="project-search" aria-labelledby="search-heading">
      <p className="eyebrow">FIND YOUR WAY</p>
      <h2 id="search-heading">Search your Project</h2>
      <label className="search-query">
        Names, aliases, Fields, connections, or writing
        <input
          type="search"
          maxLength={256}
          value={query}
          data-navigation-focus="project-search-query"
          onChange={(event) =>
            onViewChange({ ...view, query: event.currentTarget.value, limitPerGroup: 10 })
          }
        />
      </label>
      <label className="search-inactive">
        <input
          type="checkbox"
          checked={includeInactive}
          onChange={(event) =>
            onViewChange({ ...view, includeInactive: event.currentTarget.checked })
          }
        />{" "}
        Include archived and trashed records
      </label>
      {!query.trim() ? (
        <p className="muted">
          Start with a name or a few words. Names and aliases come before text matches.
        </p>
      ) : error ? (
        <div role="alert">
          <p>{error}</p>
          <button onClick={() => setAttempt((n) => n + 1)}>Retry search</button>
        </div>
      ) : !result ? (
        <p role="status">Searching…</p>
      ) : (
        <>
          <p role="status" className="muted">
            {total
              ? `${total} matching ${total === 1 ? "result" : "results"}`
              : "No matches. Try another name or fewer words."}
          </p>
          {result.groups
            .filter((group) => group.total > 0)
            .map((group) => (
              <section className="search-group" key={group.kind} aria-label={titles[group.kind]}>
                <h3>
                  {titles[group.kind]} <small className="muted">{group.total}</small>
                </h3>
                {group.kind === "text" && (
                  <p className="field-note">Text mentions are not links between records.</p>
                )}
                <ul className="search-results">
                  {group.hits.map((hit) => (
                    <li key={hit.key}>
                      <button
                        className="quiet-button search-result-title"
                        data-navigation-focus={`search-${hit.key}`}
                        onClick={() => onOpen(hit.target)}
                      >
                        {hit.title}
                      </button>
                      <span className="field-note">
                        {hit.reason}
                        {hit.workspaceState !== "active" ? ` · ${hit.workspaceState}` : ""}
                      </span>
                      <p className="field-note">{hit.context}</p>
                      {hit.excerpt && <p className="search-excerpt">{hit.excerpt}</p>}
                    </li>
                  ))}
                </ul>
                {group.hits.length < group.total && (
                  <p className="field-note">
                    Showing {group.hits.length} of {group.total}.{" "}
                    {limitPerGroup >= 100 && "Narrow your search to find more."}
                  </p>
                )}
              </section>
            ))}
          {limitPerGroup < 100 &&
            result.groups.some((group) => group.total > group.hits.length) && (
              <button
                onClick={() =>
                  onViewChange({ ...view, limitPerGroup: Math.min(100, limitPerGroup + 20) })
                }
              >
                Show more results
              </button>
            )}
        </>
      )}
    </section>
  );
}
