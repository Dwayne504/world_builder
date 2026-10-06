import type { FieldDefinition } from "./types";
import { fieldLabel, matchingFields } from "./fieldLabels";
import { useState } from "react";

export function FieldSuggestions({
  definitions,
  name,
  disabled,
  onReuse,
}: {
  definitions: FieldDefinition[];
  name: string;
  disabled?: boolean;
  onReuse: (field: FieldDefinition) => void;
}) {
  const [expanded, setExpanded] = useState({ name: "", limit: 5 });
  const limit = expanded.name === name ? expanded.limit : 5;
  const matches = matchingFields(definitions, name);
  if (!name.trim() || !matches.length) return null;
  return (
    <div className="field-suggestions">
      <strong>A Field with this name already exists.</strong>
      <p>
        Reuse it to share one definition and keep every Entry's own value. Create a separate Field
        only if it means something different.
      </p>
      <ul className="manager-bounded-list">
        {matches.slice(0, limit).map((d) => (
          <li key={d.id}>
            {fieldLabel(d, definitions)}
            <button disabled={disabled} onClick={() => onReuse(d)}>
              Reuse {d.name}
            </button>
          </li>
        ))}
      </ul>
      {matches.length > limit && (
        <div className="manager-list-footer">
          <small>
            Showing {limit} of {matches.length} matching definitions
          </small>
          <button className="quiet-button" onClick={() => setExpanded({ name, limit: limit + 5 })}>
            Show more matching fields
          </button>
        </div>
      )}
    </div>
  );
}
