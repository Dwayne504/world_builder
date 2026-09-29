import type { FieldDefinition } from "./types";
import { fieldLabel, matchingFields } from "./fieldLabels";

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
  const matches = matchingFields(definitions, name);
  if (!name.trim() || !matches.length) return null;
  return (
    <div className="field-suggestions">
      <strong>A Field with this name already exists.</strong>
      <p>
        Reuse it to share one definition and keep every Entry's own value. Create a separate Field
        only if it means something different.
      </p>
      <ul>
        {matches.map((d) => (
          <li key={d.id}>
            {fieldLabel(d, definitions)}
            <button disabled={disabled} onClick={() => onReuse(d)}>
              Reuse {d.name}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
