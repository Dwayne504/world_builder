import { useId, useState, type ReactNode } from "react";
import type { Relationship } from "./types";

export function RelationshipGroup({
  label,
  relationships,
  entryId,
  connection,
  initialOpen,
}: {
  initialOpen?: boolean;
  label: string;
  relationships: Relationship[];
  entryId: string;
  connection: (relationship: Relationship) => ReactNode;
}) {
  const [open, setOpen] = useState(initialOpen ?? false);
  const [search, setSearch] = useState("");
  const id = useId();
  const other = (r: Relationship) => (r.source.id === entryId ? r.target : r.source);
  const matches = relationships.filter((r) =>
    other(r).label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()),
  );
  const warnings = [...new Set(relationships.flatMap((r) => r.warnings))];
  return (
    <li className="relationship-group">
      <div className="relationship-group-heading">
        <span className="relationship-group-preview">
          <span>{label}</span>{" "}
          {relationships.map((r, index) => {
            const person = other(r);
            return (
              <span key={r.id}>
                {index > 0 && ", "}
                {person.label}
                {person.workspaceState !== "active" && ` (${person.workspaceState})`}
              </span>
            );
          })}
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
            onChange={(event) => setSearch(event.currentTarget.value)}
          />
        </label>
        {matches.length === 0 && <p className="muted">No Entries match this search.</p>}
        <ul className="relationship-list">{matches.map(connection)}</ul>
      </div>
    </li>
  );
}
