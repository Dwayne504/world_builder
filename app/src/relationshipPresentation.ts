import type { Relationship, RelationshipDefinition } from "./types";

export function groupRelationships(
  relationships: Relationship[],
  definitions: RelationshipDefinition[],
  entryId: string,
) {
  const groups = new Map<string, { key: string; label: string; relationships: Relationship[] }>();
  for (const relation of relationships) {
    const definition = definitions.find((item) => item.id === relation.definitionId);
    const inverse = !!definition?.directed && relation.source.id !== entryId;
    const key = `${relation.definitionId}:${inverse ? "target" : "source"}`;
    const group = groups.get(key) ?? {
      key,
      label: (inverse ? definition?.inverseLabel : definition?.forwardLabel) ?? "connects to",
      relationships: [],
    };
    group.relationships.push(relation);
    groups.set(key, group);
  }
  return [...groups.values()];
}
