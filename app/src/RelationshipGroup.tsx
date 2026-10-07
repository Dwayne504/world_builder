import { useId, useState, type ReactNode } from "react";
import type { Relationship } from "./types";

export function RelationshipGroup({
  label,
  relationships,
  entryId,
  connection,
  initialOpen,
  restoreRelationshipId,
}: {
  initialOpen?: boolean;
  restoreRelationshipId?: string;
  label: string;
  relationships: Relationship[];
  entryId: string;
  connection: (relationship: Relationship) => ReactNode;
}) {
  const [open, setOpen] = useState(initialOpen ?? false);
  const [search, setSearch] = useState("");
  const [limit, setLimit] = useState(12);
  const id = useId();
  const other = (r: Relationship) => (r.source.id === entryId ? r.target : r.source);
  const matches = relationships.filter((r) =>
    other(r).label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()),
  );
  const warnings = [...new Set(relationships.flatMap((r) => r.warnings))];
  const restored =
    !search.trim() && matches.find((relationship) => relationship.id === restoreRelationshipId);
  const visible =
    restored && !matches.slice(0, limit).some((relationship) => relationship.id === restored.id)
      ? [restored, ...matches.slice(0, limit - 1)]
      : matches.slice(0, limit);
  return (
    <li className="relationship-group">
      <div className="relationship-group-heading">
        <span className="relationship-group-preview">
          <span>{label}</span>{" "}
          {relationships.slice(0, 3).map((r, index) => {
            const person = other(r);
            return (
              <span key={r.id}>
                {index > 0 && ", "}
                {person.label}
                {person.workspaceState !== "active" && ` (${person.workspaceState})`}
              </span>
            );
          })}
          {relationships.length > 3 && <small> and {relationships.length - 3} more</small>}
        </span>
        <button
          className="quiet-button relationship-group-toggle"
          aria-expanded={open}
          aria-controls={id}
          aria-label={`${open ? "Hide" : "Show"} ${label} relationships`}
          onClick={() => setOpen((value) => !value)}
        >
          <small>{relationships.length}</small> <span aria-hidden="true">{open ? "▾" : "▸"}</span>
        </button>
      </div>
      {warnings.map((warning) => (
        <p key={warning} className="relationship-warning" role="status">
          {warning}
        </p>
      ))}
      <div id={id} hidden={!open} className="relationship-group-content">
        <label>
          Find an Entry in {label}
          <input
            type="search"
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setLimit(12);
            }}
          />
        </label>
        {matches.length === 0 && <p className="muted">No Entries match this search.</p>}
        <ul className="relationship-list">{visible.map(connection)}</ul>
        {matches.length > limit && (
          <div className="manager-list-footer">
            <small>
              Showing {visible.length} of {matches.length} connections
            </small>
            <button className="quiet-button" onClick={() => setLimit(limit + 12)}>
              Show more {label} connections
            </button>
          </div>
        )}
      </div>
    </li>
  );
}
