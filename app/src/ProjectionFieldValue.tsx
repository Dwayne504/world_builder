import { useEffect, useState } from "react";
import { listCategories, readProjectRelationships } from "./api";
import type { Category, EntryField, FieldCommand, RelationshipSnapshot } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export function ProjectionFieldValue({
  projectId,
  entryId,
  field,
  disabled,
  onCommand,
  onDirty,
  onNavigate,
}: {
  projectId: string;
  entryId: string;
  field: EntryField;
  disabled: boolean;
  onCommand: (command: FieldCommand) => Promise<SubmitOutcome>;
  onDirty: (fieldId: string, dirty: boolean) => void;
  onNavigate?: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [instanceId, setInstanceId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [limit, setLimit] = useState(10);
  const [creating, setCreating] = useState(false);
  const [categoryId, setCategoryId] = useState("");
  const [catalog, setCatalog] = useState<RelationshipSnapshot | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const relationships = field.projectedRelationships ?? [];
  useEffect(() => {
    onDirty(field.definition.id, editing);
    return () => onDirty(field.definition.id, false);
  }, [editing, field.definition.id, onDirty]);
  useEffect(() => {
    if (!editing) return;
    let current = true;
    setLoading(true);
    void Promise.all([readProjectRelationships(projectId), listCategories(projectId)]).then(
      ([snapshot, nextCategories]) => {
        if (current) {
          setCatalog(snapshot);
          setCategories(nextCategories);
          setLoading(false);
          setError(null);
        }
      },
      (reason: unknown) => {
        if (current) {
          setLoading(false);
          setError(reason instanceof Error ? reason.message : "Entries could not be loaded.");
        }
      },
    );
    return () => {
      current = false;
    };
  }, [editing, projectId, retry]);
  function edit(id: string | null) {
    setInstanceId(id);
    setQuery("");
    setLimit(10);
    setCreating(false);
    setCategoryId("");
    setError(null);
    setEditing(true);
  }
  async function save(other: Extract<FieldCommand, { kind: "edit_projection" }>["other"]) {
    const result = await onCommand({
      kind: "edit_projection",
      fieldId: field.definition.id,
      instanceId,
      other,
    });
    if (result.kind === "committed") setEditing(false);
  }
  const results = query.trim()
    ? (catalog?.entries ?? []).filter((entry) =>
        `${entry.label} ${entry.categoryName}`
          .toLocaleLowerCase()
          .includes(query.trim().toLocaleLowerCase()),
      )
    : [];
  return (
    <div className="projection-value">
      {!relationships.length && <span className="muted">Not connected</span>}
      {relationships.map((relationship) => {
        const participant =
          relationship.source.id === entryId ? relationship.target : relationship.source;
        return (
          <div className="projection-target" key={relationship.id}>
            <div className="row">
              {participant.id && onNavigate ? (
                <button
                  className="quiet-button relationship-entry-link"
                  data-navigation-focus={`projection-${field.definition.id}-${relationship.id}`}
                  disabled={disabled || editing}
                  onClick={() => onNavigate(participant.id!)}
                >
                  {participant.label}
                </button>
              ) : (
                <span>{participant.label}</span>
              )}
              {participant.workspaceState !== "active" && (
                <small className="muted">{participant.workspaceState}</small>
              )}
              <button
                className="quiet-button"
                aria-label={`Change ${field.definition.name}: ${participant.label}`}
                disabled={disabled || editing || !field.available}
                onClick={() => edit(relationship.id)}
              >
                Change
              </button>
            </div>
            {relationship.warnings.map((warning) => (
              <p className="field-note" role="status" key={warning}>
                {warning}
              </p>
            ))}
            {relationship.note && (
              <details className="projection-note">
                <summary>Note</summary>
                <p>{relationship.note}</p>
              </details>
            )}
          </div>
        );
      })}
      {!relationships.length && !editing && (
        <button
          className="quiet-button"
          disabled={disabled || !field.available}
          onClick={() => edit(null)}
        >
          Choose {field.definition.name}
        </button>
      )}
      {!field.available && (
        <p className="field-note">
          Restore this Field in Manage fields before changing its connection.
        </p>
      )}
      {relationships.length > 1 && (
        <p className="field-note">
          Choose a connection to change. Use Relationships to end a connection or keep both.
        </p>
      )}
      {editing && (
        <fieldset className="projection-editor" disabled={disabled}>
          <legend>Change {field.definition.name}</legend>
          <label>
            Find an Entry
            <input
              aria-label={`Find Entry for ${field.definition.name}`}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setCreating(false);
                setLimit(10);
              }}
            />
          </label>
          {loading && <p role="status">Loading Entries…</p>}
          {error && (
            <p role="alert">
              {error} <button onClick={() => setRetry((n) => n + 1)}>Retry loading Entries</button>
            </p>
          )}
          {!loading && !error && !creating && (
            <>
              {!query.trim() && (
                <p className="field-note">Search by name, or type a name to create an Entry.</p>
              )}
              <ul className="projection-results">
                {results.slice(0, limit).map((entry) => (
                  <li key={entry.id}>
                    <button onClick={() => void save({ kind: "existing", id: entry.id })}>
                      {entry.label} <small>{entry.categoryName}</small>
                    </button>
                  </li>
                ))}
              </ul>
              {results.length > limit && (
                <button
                  className="quiet-button"
                  onClick={() => setLimit((current) => current + 10)}
                >
                  Show more matches ({limit} of {results.length})
                </button>
              )}
              {query.trim() && (
                <button onClick={() => setCreating(true)}>Create “{query.trim()}”</button>
              )}
            </>
          )}
          {creating && (
            <>
              <label>
                Category
                <select
                  aria-label="New connected Entry Category"
                  value={categoryId}
                  onChange={(event) => setCategoryId(event.target.value)}
                >
                  <option value="">Uncategorized</option>
                  {categories
                    .filter((category) => !category.isUncategorized)
                    .map((category) => (
                      <option key={category.id} value={category.id}>
                        {category.name}
                      </option>
                    ))}
                </select>
              </label>
              <button
                disabled={!query.trim()}
                onClick={() =>
                  void save({ kind: "create", name: query.trim(), categoryId: categoryId || null })
                }
              >
                Create and connect
              </button>
            </>
          )}
          <button className="quiet-button" onClick={() => setEditing(false)}>
            Cancel connection change
          </button>
        </fieldset>
      )}
    </div>
  );
}
