import { useEffect, useState } from "react";
import { readProjectRelationships } from "./api";
import type { FieldProjection, RelationshipDefinition } from "./types";
import { ManagerSearchSelect } from "./ManagerSearchSelect";

/** Configuration is only a view of the existing Relationship definition. */
export function ProjectionConfiguration({
  projectId,
  value,
  onChange,
}: {
  projectId: string;
  value: FieldProjection;
  onChange: (value: FieldProjection) => void;
}) {
  const [definitions, setDefinitions] = useState<RelationshipDefinition[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let current = true;
    void readProjectRelationships(projectId).then(
      (snapshot) => {
        if (current) {
          setDefinitions(snapshot.definitions.filter((definition) => !definition.retired));
          setError(null);
        }
      },
      (reason: unknown) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "Relationships could not be loaded.");
      },
    );
    return () => {
      current = false;
    };
  }, [projectId, retry]);
  const selected = definitions?.find(
    (definition) => definition.id === value.relationshipDefinitionId,
  );
  return (
    <div className="projection-configuration">
      <p className="field-note">
        Show a connection as a Field. Editing it updates the same relationship.
      </p>
      {error && (
        <p role="alert">
          {error}{" "}
          <button onClick={() => setRetry((n) => n + 1)}>Retry loading relationships</button>
        </p>
      )}
      {!definitions && !error && <p role="status">Loading relationships…</p>}
      {definitions?.length === 0 && (
        <p>No relationship definitions yet. Add one in Manage relationships, then return here.</p>
      )}
      <ManagerSearchSelect
        label="Relationship"
        ariaLabel="Field relationship"
        value={value.relationshipDefinitionId}
        onChange={(id) => onChange({ relationshipDefinitionId: id, perspective: "source" })}
        emptyLabel="Choose a relationship"
        choices={(definitions ?? []).map((definition) => ({
          id: definition.id,
          label: `${definition.name} · ${definition.forwardLabel}${definition.directed ? ` / ${definition.inverseLabel}` : ""}`,
        }))}
      />
      {selected?.directed && (
        <label>
          This Entry…
          <select
            aria-label="Field relationship direction"
            value={value.perspective}
            onChange={(event) =>
              onChange({
                ...value,
                perspective: event.target.value as FieldProjection["perspective"],
              })
            }
          >
            <option value="source">{selected.forwardLabel}</option>
            <option value="target">{selected.inverseLabel}</option>
          </select>
        </label>
      )}
    </div>
  );
}
