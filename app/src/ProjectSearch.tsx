import { useEffect, useId, useState } from "react";
import { searchProject } from "./api";
import type { SearchHit, SearchResults, SearchTarget, SearchView } from "./searchTypes";

const areaNames = { manuscript: "Manuscript", plan: "Plan", notes: "Notes" };

const titles = {
  timeline: "Timeline",
  entries: "Entries",
  chapters: "Chapters",
  roles: "Story Roles",
  structured: "Fields, connections & linked Chapters",
  text: "Manuscript & other text",
};

function ResultRow({
  hit,
  view,
  onViewChange,
  onOpen,
  textMatch,
}: {
  hit: SearchHit;
  view: SearchView;
  onViewChange: (view: SearchView) => void;
  onOpen: (target: SearchTarget) => void;
  textMatch: boolean;
}) {
  const bodyId = useId();
  const isChapter = hit.target.kind === "chapter";
  const expanded = view.expandedHits?.includes(hit.key) ?? false;
  const extended = view.extendedHits?.includes(hit.key) ?? false;
  const toggle = (property: "expandedHits" | "extendedHits") => {
    const keys = view[property] ?? [];
    onViewChange({
      ...view,
      [property]: keys.includes(hit.key)
        ? keys.filter((key) => key !== hit.key)
        : [...keys, hit.key],
    });
  };
  const resultContext =
    textMatch && hit.target.kind === "chapter"
      ? `Preview from ${areaNames[hit.target.area]}`
      : hit.context;
  const context = [resultContext, hit.workspaceState === "active" ? "" : hit.workspaceState]
    .filter(Boolean)
    .join(" · ");
  const preview = (
    <>
      {hit.excerpt && (
        <p className={`search-excerpt${extended ? " is-extended" : ""}`}>
          {extended ? hit.preview : hit.excerpt}
        </p>
      )}
      {hit.preview && hit.preview !== hit.excerpt && (
        <button
          className="quiet-button search-preview-toggle"
          aria-expanded={extended}
          onClick={() => toggle("extendedHits")}
        >
          {extended ? "Shorter preview" : "Longer preview"}
        </button>
      )}
    </>
  );
  return (
    <li>
      {isChapter ? (
        <>
          <button
            className="search-chapter-toggle"
            aria-expanded={expanded}
            aria-controls={bodyId}
            data-navigation-focus={`search-${hit.key}`}
            onClick={() => toggle("expandedHits")}
          >
            <span className="search-disclosure-icon" aria-hidden="true">
              {expanded ? "▾" : "▸"}
            </span>
            <span className="search-result-label">
              <span className="search-result-name" title={hit.title}>
                {hit.title}
              </span>
              <small>{context}</small>
            </span>
          </button>
          {expanded && (
            <div className="search-result-body" id={bodyId}>
              {preview}
              <button className="quiet-button" onClick={() => onOpen(hit.target)}>
                Open Chapter
                {hit.target.kind === "chapter" ? ` · ${areaNames[hit.target.area]}` : ""}
              </button>
            </div>
          )}
        </>
      ) : (
        <>
          <div className="search-result-heading">
            <button
              className="quiet-button search-result-title"
              title={hit.title}
              data-navigation-focus={`search-${hit.key}`}
              onClick={() => onOpen(hit.target)}
            >
              {hit.title}
            </button>
            <small className="muted">{context}</small>
          </div>
          {hit.reason.startsWith("Alias:") && <small className="muted">{hit.reason}</small>}
          {hit.target.kind === "relationship"
            ? hit.excerpt && (
                <>
                  <button
                    className="quiet-button search-note-toggle"
                    aria-expanded={expanded}
                    aria-controls={bodyId}
                    onClick={() => toggle("expandedHits")}
                  >
                    {expanded ? "▾" : "▸"} Note
                  </button>
                  {expanded && (
                    <div id={bodyId} className="search-result-body">
                      {preview}
                    </div>
                  )}
                </>
              )
            : preview}
        </>
      )}
    </li>
  );
}

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
  const { query, includeInactive, limitPerGroup, entryId, structuredKind, textArea, storyRoleId } =
    view;
  const [count, setCount] = useState(String(limitPerGroup));
  const [countError, setCountError] = useState(false);
  useEffect(() => {
    setCount(String(limitPerGroup));
  }, [limitPerGroup]);
  function applyCount() {
    const value = Number(count);
    const valid = Number.isSafeInteger(value) && value > 0;
    setCountError(!valid);
    if (valid && value !== limitPerGroup) onViewChange({ ...view, limitPerGroup: value });
  }
  useEffect(() => {
    let current = true;
    setResult(null);
    setError(null);
    if (!query.trim() && !entryId && !storyRoleId) return;
    const timer = window.setTimeout(() => {
      void searchProject(projectId, {
        query,
        includeInactive,
        limitPerGroup,
        ...(entryId ? { entryId } : {}),
        ...(structuredKind ? { structuredKind } : {}),
        ...(textArea ? { textArea } : {}),
        ...(storyRoleId ? { storyRoleId } : {}),
      })
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
  }, [
    projectId,
    query,
    includeInactive,
    limitPerGroup,
    entryId,
    structuredKind,
    textArea,
    storyRoleId,
    attempt,
  ]);
  const total = result?.groups.reduce((sum, group) => sum + group.total, 0) ?? 0;
  return (
    <section className="project-search" aria-labelledby="search-heading">
      <p className="eyebrow">FIND YOUR WAY</p>
      <h2 id="search-heading">
        {storyRoleId
          ? `Chapters using ${view.storyRoleName ?? "this Role"}`
          : entryId
            ? `Search within ${view.entryName ?? "this Entry"}`
            : "Search your Project"}
      </h2>
      {storyRoleId && (
        <p className="search-scope muted">
          Entries assigned this Role on Chapter links.{" "}
          <button
            className="quiet-button"
            onClick={() =>
              onViewChange({
                ...view,
                storyRoleId: undefined,
                storyRoleName: undefined,
                expandedHits: [],
                extendedHits: [],
              })
            }
          >
            Clear Role filter
          </button>
        </p>
      )}
      {entryId && (
        <p className="search-scope muted">
          Fields, connections, and explicitly linked Chapters.{" "}
          <button
            className="quiet-button"
            onClick={() =>
              onViewChange({
                ...view,
                entryId: undefined,
                entryName: undefined,
                expandedHits: [],
                extendedHits: [],
              })
            }
          >
            Search whole Project
          </button>
        </p>
      )}
      <label className="search-query">
        {storyRoleId
          ? "Find a linked Entry or Chapter"
          : entryId
            ? "Find within this Entry"
            : "Names, aliases, Roles, Fields, connections, or writing"}
        <input
          type="search"
          maxLength={256}
          value={query}
          data-navigation-focus="project-search-query"
          onChange={(event) =>
            onViewChange({
              ...view,
              query: event.currentTarget.value,
              expandedHits: [],
              extendedHits: [],
            })
          }
        />
      </label>
      <div className="search-controls">
        <label hidden={!!storyRoleId}>
          Fields & connections filter
          <select
            disabled={!!storyRoleId}
            aria-label="Fields & connections filter"
            value={structuredKind ?? "all"}
            onChange={(event) =>
              onViewChange({
                ...view,
                structuredKind:
                  event.currentTarget.value === "all"
                    ? undefined
                    : (event.currentTarget.value as SearchView["structuredKind"]),
              })
            }
          >
            <option value="all">All Fields, connections & Chapter links</option>
            <option value="fields">Fields</option>
            <option value="relationships">Relationships</option>
            <option value="chapters">Linked Chapters</option>
          </select>
        </label>
        <label hidden={!!storyRoleId}>
          Text previews
          <select
            disabled={!!storyRoleId}
            aria-label="Text previews"
            value={textArea ?? "all"}
            onChange={(event) =>
              onViewChange({
                ...view,
                textArea:
                  event.currentTarget.value === "all"
                    ? undefined
                    : (event.currentTarget.value as SearchView["textArea"]),
              })
            }
          >
            <option value="all">All matching writing areas</option>
            <option value="manuscript">Manuscript</option>
            <option value="plan">Plan</option>
            <option value="notes">Notes</option>
          </select>
        </label>
        <label className="search-count">
          Results per section
          <input
            type="number"
            min="1"
            step="1"
            value={count}
            aria-invalid={countError}
            onChange={(event) => {
              setCount(event.currentTarget.value);
              setCountError(false);
            }}
            onBlur={applyCount}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                applyCount();
              }
            }}
          />
        </label>
        <label className="search-inactive">
          <input
            type="checkbox"
            checked={includeInactive}
            onChange={(event) =>
              onViewChange({ ...view, includeInactive: event.currentTarget.checked })
            }
          />
          Include archived and trashed records
        </label>
      </div>
      {countError && <p role="alert">Choose a whole number of results, starting at 1.</p>}
      {!query.trim() && !entryId && !storyRoleId ? (
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
              : storyRoleId
                ? "No matching Chapter links. To use this Role, link an Entry in a Chapter and choose Roles beside its name. Check your search and archived-record filter too."
                : "No matches. Try another name, filter, or fewer words."}
          </p>
          {result.groups
            .filter((group) => group.total > 0)
            .map((group) => (
              <section
                className="search-group"
                key={group.kind}
                aria-label={
                  storyRoleId && group.kind === "structured" ? "Chapter uses" : titles[group.kind]
                }
              >
                <h3>
                  {storyRoleId && group.kind === "structured" ? "Chapter uses" : titles[group.kind]}{" "}
                  <small className="muted">{group.total}</small>
                </h3>
                {group.kind === "roles" && (
                  <p className="field-note">
                    Choose a Role to find its Chapter uses. Creating a Role makes it available;
                    assign it to a linked Entry to use it.
                  </p>
                )}
                {group.kind === "text" && (
                  <p className="field-note">
                    Previews come from the writing area matching your search. Change it with Text
                    previews above. Text matches are separate from Chapter links.
                  </p>
                )}
                <ul className="search-results">
                  {group.hits.map((hit) => (
                    <ResultRow
                      key={hit.key}
                      hit={hit}
                      view={view}
                      onViewChange={onViewChange}
                      onOpen={onOpen}
                      textMatch={group.kind === "text"}
                    />
                  ))}
                </ul>
                {group.hits.length < group.total && (
                  <p className="field-note">
                    Showing {group.hits.length} of {group.total}.
                  </p>
                )}
              </section>
            ))}
          {result.groups.some((group) => group.total > group.hits.length) && (
            <button
              onClick={() =>
                onViewChange({
                  ...view,
                  limitPerGroup: Math.min(
                    Math.max(...result.groups.map((group) => group.total)),
                    limitPerGroup + 20,
                  ),
                })
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
