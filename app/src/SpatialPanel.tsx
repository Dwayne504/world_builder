import { useCallback, useEffect, useRef, useState } from "react";
import { applySpatial, readSpatial } from "./api";
import { Dialog } from "./Dialog";
import { ManagerSearchSelect } from "./ManagerSearchSelect";
import type { FieldsController } from "./EntryFieldsPanel";
import type { Category, SpatialCommand, SpatialSnapshot, RelationshipSnapshot } from "./types";
import type { SubmitOutcome } from "./useProjectRename";
import { spatialDescendants, spatialPath } from "./spatialPresentation";

export function SpatialPanel({
  projectId,
  entryId,
  categories,
  disabled,
  onController,
  onRevision,
  getRevision,
  onNavigate,
  onEntriesChanged,
  onCommitted,
  refreshKey,
  relations,
}: {
  projectId: string;
  entryId: string;
  categories: Category[];
  disabled: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision: () => number;
  onNavigate: (id: string) => void;
  onEntriesChanged: () => void;
  onCommitted: () => void;
  refreshKey: number;
  relations?: RelationshipSnapshot | null;
}) {
  const [snapshot, setSnapshot] = useState<SpatialSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [manage, setManage] = useState(false);
  const [form, setForm] = useState<"parent" | "child" | "new" | null>(null);
  const [query, setQuery] = useState("");
  const [childQuery, setChildQuery] = useState("");
  const [name, setName] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [all, setAll] = useState(false);
  const [limit, setLimit] = useState(20);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const generation = useRef(0);
  const dirty = form === "new" && !!(name || categoryId);
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  const failedRef = useRef(failed);
  failedRef.current = failed;
  const submit = useCallback(async (): Promise<SubmitOutcome> => {
    if (pending.current) return pending.current;
    return { kind: dirtyRef.current || failedRef.current ? "failed" : "no-op" };
  }, []);
  useEffect(
    () =>
      onController({
        state: busy ? "saving" : failed ? "failed" : dirty ? "dirty" : "saved",
        submit,
        canSubmit: !dirty,
        waitForPending: submit,
      }),
    [busy, failed, dirty, onController, submit],
  );
  const reload = useCallback(async () => {
    const token = ++generation.current;
    setLoading(true);
    try {
      const next = await readSpatial(projectId);
      if (token !== generation.current) return;
      setSnapshot(next);
      setError(null);
      setFailed(false);
    } catch (reason) {
      if (token === generation.current)
        setError(
          reason instanceof Error ? reason.message : "Spatial structure could not be loaded.",
        );
    } finally {
      if (token === generation.current) setLoading(false);
    }
  }, [projectId]);
  useEffect(() => {
    if (!pending.current && !dirtyRef.current && !failedRef.current) void reload();
  }, [reload, refreshKey]);
  useEffect(
    () => () => {
      ++generation.current;
    },
    [],
  );
  function reset() {
    setForm(null);
    setName("");
    setCategoryId("");
    setQuery("");
    dirtyRef.current = false;
  }
  function perform(command: SpatialCommand) {
    if (!snapshot || pending.current || disabled || loading) return;
    ++generation.current;
    setBusy(true);
    setError(null);
    pending.current = applySpatial(
      projectId,
      Math.max(snapshot.globalRevision, getRevision()),
      command,
    )
      .then((next): SubmitOutcome => {
        setSnapshot(next);
        if (command.kind === "set_enabled" && !command.enabled) setManage(false);
        reset();
        setFailed(false);
        failedRef.current = false;
        onRevision(next.globalRevision);
        onCommitted();
        if (command.kind === "create_child") onEntriesChanged();
        return { kind: "committed" };
      })
      .catch((reason: unknown): SubmitOutcome => {
        setError(
          reason instanceof Error ? reason.message : "The Spatial change could not be saved.",
        );
        setFailed(true);
        failedRef.current = true;
        return { kind: "failed" };
      })
      .finally(() => {
        pending.current = null;
        setBusy(false);
      });
  }
  const entries = snapshot?.entries ?? [];
  const current = entries.find((e) => e.id === entryId);
  const descendants = spatialDescendants(entries, entryId);
  const path = spatialPath(entries, entryId);
  const ancestors = new Set(path.map((e) => e.id));
  const relatedPlaces = (relations?.relationships ?? [])
    .filter((r) => !r.ended && r.workspaceState === "active")
    .flatMap((r) => {
      const fromHere = r.source.id === entryId;
      const other = fromHere ? r.target : r.source;
      const definition = relations?.definitions.find((d) => d.id === r.definitionId);
      const context = other.id ? spatialPath(entries, other.id) : [];
      return context.length && definition
        ? [
            {
              id: r.id,
              context,
              label: fromHere ? definition.forwardLabel : definition.inverseLabel,
            },
          ]
        : [];
    });
  const children = entries.filter((e) => (all ? descendants.has(e.id) : e.parentId === entryId));
  const matching = children.filter((e) =>
    e.label.toLocaleLowerCase().includes(childQuery.toLocaleLowerCase()),
  );
  const candidates = entries.filter(
    (e) =>
      e.spatial &&
      e.workspaceState === "active" &&
      e.id !== entryId &&
      (form === "parent"
        ? !descendants.has(e.id) && e.id !== current?.parentId
        : !ancestors.has(e.id) && e.parentId !== entryId) &&
      e.label.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  const locked = busy || disabled || loading;
  const feedback = (
    <>
      {loading && <p role="status">Loading Spatial structure…</p>}
      {error && (
        <p role="alert">
          {error}{" "}
          <button disabled={busy || disabled} onClick={() => void reload()}>
            Reload Spatial structure (keep draft)
          </button>
        </p>
      )}
    </>
  );
  return (
    <section className="spatial-panel" aria-label="Spatial structure">
      {!manage && feedback}
      {!current?.spatial ? (
        <details>
          <summary>Spatial feature</summary>
          <p>
            Let this Entry contain places or objects in a hierarchy. Its Category and Type stay the
            same.
          </p>
          <button
            disabled={locked || !current}
            onClick={() => perform({ kind: "set_enabled", entryId, enabled: true })}
          >
            Enable Spatial
          </button>
        </details>
      ) : (
        <>
          <div className="section-heading">
            <h3>Spatial structure</h3>
            <button disabled={locked} className="quiet-button" onClick={() => setManage(true)}>
              Arrange places
            </button>
          </div>
          <nav aria-label="Spatial breadcrumbs" className="spatial-breadcrumbs">
            {path.map((e, i) => (
              <span key={e.id}>
                {i > 0 && " › "}
                {e.id === entryId ? (
                  e.label
                ) : (
                  <button
                    className="link-button"
                    disabled={locked || dirty}
                    onClick={() => onNavigate(e.id)}
                  >
                    {e.label}
                  </button>
                )}
                {e.workspaceState !== "active" && ` (${e.workspaceState})`}
              </span>
            ))}
          </nav>
          <div className="spatial-filters">
            <label>
              Contents
              <select
                value={all ? "all" : "direct"}
                onChange={(e) => {
                  setAll(e.target.value === "all");
                  setLimit(20);
                }}
              >
                <option value="direct">Direct children</option>
                <option value="all">All descendants</option>
              </select>
            </label>
            <label>
              Find a place inside
              <input
                type="search"
                value={childQuery}
                onChange={(e) => {
                  setChildQuery(e.target.value);
                  setLimit(20);
                }}
              />
            </label>
          </div>
          {!matching.length && (
            <p className="muted">
              {children.length ? "No matching places." : "No places inside yet."}
            </p>
          )}
          <ul className="spatial-list">
            {matching.slice(0, limit).map((e) => (
              <li key={e.id}>
                <button
                  className="link-button"
                  disabled={locked || dirty}
                  onClick={() => onNavigate(e.id)}
                >
                  {e.label}
                </button>
                {e.workspaceState !== "active" && <small>{e.workspaceState}</small>}
                {all && (
                  <small>
                    {spatialPath(entries, e.id)
                      .slice(0, -1)
                      .map((p) => p.label)
                      .join(" › ")}
                  </small>
                )}
              </li>
            ))}
          </ul>
          {matching.length > limit && (
            <button onClick={() => setLimit(limit + 20)}>
              Show more places ({matching.length - limit} remaining)
            </button>
          )}
        </>
      )}
      {relatedPlaces.length > 0 && (
        <details className="spatial-context">
          <summary>Related places</summary>
          <p className="muted">
            Relationships stay direct. These paths show the places around their targets.
          </p>
          <ul className="spatial-list">
            {relatedPlaces.map((related) => (
              <li key={related.id}>
                <span>
                  {related.label}{" "}
                  <strong>{related.context[related.context.length - 1]?.label}</strong>
                  <small>
                    {related.context.map((place, i) => (
                      <span key={place.id}>
                        {i > 0 && " › "}
                        <button
                          className="link-button"
                          disabled={locked || dirty}
                          onClick={() => onNavigate(place.id)}
                        >
                          {place.label}
                        </button>
                        {place.workspaceState !== "active" && ` (${place.workspaceState})`}
                      </span>
                    ))}
                  </small>
                </span>
              </li>
            ))}
          </ul>
        </details>
      )}
      <Dialog
        open={manage}
        title="Arrange places"
        onClose={() => {
          if (!busy) setManage(false);
        }}
      >
        <p>
          This is structural containment. Current location, travel, and ownership belong in
          Relationships.
        </p>
        {manage && feedback}
        <fieldset disabled={locked}>
          <p>
            Inside:{" "}
            <strong>{entries.find((e) => e.id === current?.parentId)?.label ?? "Top level"}</strong>
          </p>
          <div className="row">
            <button
              disabled={dirty}
              onClick={() => {
                setForm("parent");
                setQuery("");
              }}
            >
              Change parent
            </button>
            <button
              disabled={dirty}
              onClick={() => {
                setForm("child");
                setQuery("");
              }}
            >
              Add existing child
            </button>
            <button onClick={() => setForm("new")}>Create child</button>
          </div>
          {form === "parent" && (
            <button
              onClick={() => perform({ kind: "reparent", entryId, parentId: null })}
              disabled={!current?.parentId}
            >
              Move to top level
            </button>
          )}
          {(form === "parent" || form === "child") && (
            <>
              <label>
                {form === "parent" ? "Find a parent" : "Find an existing child"}
                <input type="search" value={query} onChange={(e) => setQuery(e.target.value)} />
              </label>
              <p className="muted">
                Moving an Entry brings its children with it. Only Entries with Spatial enabled are
                listed.
              </p>
              <ul className="spatial-list">
                {candidates.slice(0, 10).map((e) => (
                  <li key={e.id}>
                    <span>
                      {e.label}
                      <small>
                        {spatialPath(entries, e.id)
                          .slice(0, -1)
                          .map((p) => p.label)
                          .join(" › ") || "Top level"}
                      </small>
                    </span>
                    <button
                      onClick={() =>
                        perform({
                          kind: "reparent",
                          entryId: form === "parent" ? entryId : e.id,
                          parentId: form === "parent" ? e.id : entryId,
                        })
                      }
                    >
                      {form === "parent" ? `Move inside ${e.label}` : `Move ${e.label} here`}
                    </button>
                  </li>
                ))}
              </ul>
              {!candidates.length && <p>No matching places available.</p>}
              {candidates.length > 10 && (
                <p>Showing 10 of {candidates.length}. Refine your search.</p>
              )}
            </>
          )}
          {form === "new" && (
            <>
              <label>
                Child name (optional)
                <input value={name} onChange={(e) => setName(e.target.value)} />
              </label>
              <ManagerSearchSelect
                label="Child Category"
                value={categoryId}
                onChange={setCategoryId}
                emptyLabel="Uncategorized"
                choices={categories
                  .filter((c) => !c.isUncategorized)
                  .map((c) => ({ id: c.id, label: c.name }))}
              />
              <button
                onClick={() =>
                  perform({
                    kind: "create_child",
                    parentId: entryId,
                    name: name || null,
                    categoryId: categoryId || null,
                    typeId: null,
                  })
                }
              >
                Create place inside {current?.label}
              </button>
            </>
          )}
          {form && (
            <button className="quiet-button" onClick={reset}>
              Cancel Spatial draft
            </button>
          )}
          <details>
            <summary>Remove Spatial feature</summary>
            <p>
              Move this Entry to the top level and its children elsewhere first. The Entry and its
              relationships are kept.
            </p>
            <button
              disabled={dirty || !!current?.parentId || descendants.size > 0}
              onClick={() => perform({ kind: "set_enabled", entryId, enabled: false })}
            >
              Remove Spatial
            </button>
          </details>
        </fieldset>
      </Dialog>
      {!manage && dirty && (
        <p>
          Spatial has an unfinished child.{" "}
          <button onClick={() => setManage(true)}>Continue Spatial draft</button>
        </p>
      )}
    </section>
  );
}
