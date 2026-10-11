import { useEffect, useState } from "react";
import { listEntries } from "./api";
import { ChapterPagination } from "./ChapterPagination";
import type { Entry, EntryField, ReferenceSelection } from "./types";

/** Plain pointers share the Field save/close guard, not Relationship semantics. */
export function EntryReferenceValue({
  projectId,
  field,
  draft,
  disabled,
  onChange,
  onNavigate,
}: {
  projectId: string;
  field: EntryField;
  draft: string;
  disabled: boolean;
  onChange: (draft: string) => void;
  onNavigate?: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [query, setQuery] = useState("");
  const [selectedQuery, setSelectedQuery] = useState("");
  const [page, setPage] = useState(0);
  const [selectedPage, setSelectedPage] = useState(0);
  useEffect(() => {
    if (!editing) return;
    let current = true;
    setError(null);
    void listEntries(projectId).then(
      (values) => {
        if (current) setEntries(values);
      },
      (reason) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "Entries could not be loaded.");
      },
    );
    return () => {
      current = false;
    };
  }, [editing, projectId, retry]);
  const selection: ReferenceSelection = draft ? JSON.parse(draft) : { expectedIds: [], items: [] };
  const targets = field.referenceTargets ?? [];
  const selected = selection.items.map((item) => {
    if (item.kind === "existing") {
      const target = targets.find((t) => t.id === item.referenceId);
      return {
        item,
        key: item.referenceId,
        id: target?.targetId ?? null,
        label: target?.label ?? "Unavailable reference",
        state: target?.workspaceState ?? null,
      };
    }
    const target = entries.find((e) => e.id === item.entryId);
    return {
      item,
      key: item.entryId,
      id: item.entryId,
      label: target?.displayName ?? "Selected Entry",
      state: target?.workspaceState ?? "active",
    };
  });
  const selectedIds = new Set(selected.map((item) => item.id));
  const matching = entries.filter(
    (e) =>
      e.workspaceState === "active" &&
      !selectedIds.has(e.id) &&
      e.displayName.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const shown = selected.filter((e) =>
    e.label.toLocaleLowerCase().includes(selectedQuery.trim().toLocaleLowerCase()),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matching.length / 10) - 1));
  const currentSelectedPage = Math.min(selectedPage, Math.max(0, Math.ceil(shown.length / 10) - 1));
  function update(items: ReferenceSelection["items"]) {
    onChange(JSON.stringify({ ...selection, items }));
  }
  return (
    <div className="entry-reference-value">
      {!selected.length && <span className="muted">Not filled in</span>}
      {selected.length > 10 && (
        <label>
          Find selected Entries
          <input
            type="search"
            value={selectedQuery}
            onChange={(e) => {
              setSelectedQuery(e.target.value);
              setSelectedPage(0);
            }}
          />
        </label>
      )}
      <ul className="manager-types" aria-label={`Selected Entries: ${field.definition.name}`}>
        {shown.slice(currentSelectedPage * 10, (currentSelectedPage + 1) * 10).map((row) => (
          <li key={row.key}>
            <span>
              {row.id && onNavigate ? (
                <button
                  className="quiet-button relationship-entry-link"
                  onClick={() => onNavigate(row.id!)}
                >
                  {row.label}
                </button>
              ) : (
                row.label
              )}
              {row.state !== "active" && <small> · {row.state ?? "Unavailable"}</small>}
            </span>
            {editing && (
              <span className="row">
                <button
                  className="quiet-button"
                  disabled={disabled || selected[0]?.key === row.key}
                  aria-label={`Move up: ${row.label}`}
                  onClick={() => {
                    const items = [...selection.items];
                    const index = selected.findIndex((s) => s.key === row.key);
                    [items[index - 1], items[index]] = [items[index], items[index - 1]];
                    update(items);
                  }}
                >
                  ↑
                </button>
                <button
                  className="quiet-button"
                  disabled={disabled}
                  aria-label={`Remove reference: ${row.label}`}
                  onClick={() => update(selection.items.filter((item) => item !== row.item))}
                >
                  Remove
                </button>
              </span>
            )}
          </li>
        ))}
      </ul>
      <ChapterPagination
        label={`selected ${field.definition.name} Entries`}
        page={currentSelectedPage}
        pageSize={10}
        total={shown.length}
        onPage={setSelectedPage}
      />
      <details open={editing} onToggle={(e) => setEditing(e.currentTarget.open)}>
        <summary>Edit references</summary>
        <p className="field-note">
          Choose one or more Entries. These links do not create Relationships.
        </p>
        {draft !==
          (field.value?.kind === "entry_references" ? JSON.stringify(field.value.value) : "") && (
          <button
            className="quiet-button"
            disabled={disabled}
            onClick={() =>
              onChange(
                JSON.stringify(
                  field.value?.kind === "entry_references"
                    ? field.value.value
                    : { expectedIds: [], items: [] },
                ),
              )
            }
          >
            Use saved references
          </button>
        )}
        {error && (
          <p role="alert">
            {error} <button onClick={() => setRetry((n) => n + 1)}>Retry Entry list</button>
          </p>
        )}
        <label>
          Find an Entry
          <input
            type="search"
            aria-label={`Find Entry for ${field.definition.name}`}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <ul className="manager-types" aria-label={`Available Entries: ${field.definition.name}`}>
          {matching.slice(currentPage * 10, (currentPage + 1) * 10).map((entry) => (
            <li key={entry.id}>
              <span>{entry.displayName}</span>
              <button
                className="quiet-button"
                disabled={disabled}
                aria-label={`Add reference: ${entry.displayName}`}
                onClick={() => update([...selection.items, { kind: "entry", entryId: entry.id }])}
              >
                Add
              </button>
            </li>
          ))}
        </ul>
        {!error && !matching.length && <p className="muted">No matching Entries.</p>}
        <ChapterPagination
          label={`available ${field.definition.name} Entries`}
          page={currentPage}
          pageSize={10}
          total={matching.length}
          onPage={setPage}
        />
      </details>
    </div>
  );
}
