import { expect, it } from "vitest";
import { groupRelationships } from "./relationshipPresentation";
import type { Relationship, RelationshipDefinition } from "./types";
const person = (id: string) => ({ id, label: id, workspaceState: "active" });
const definition: RelationshipDefinition = {
  id: "d",
  name: "Meaning",
  forwardLabel: "knows",
  inverseLabel: "knows",
  directed: false,
  retired: false,
  revision: 1,
  expectedSourcesPerTarget: null,
  expectedTargetsPerSource: null,
};
const relation = (
  id: string,
  source: string,
  target: string,
  definitionId = "d",
): Relationship => ({
  id,
  source: person(source),
  target: person(target),
  definitionId,
  note: id,
  ended: false,
  workspaceState: "active",
  revision: 1,
  warnings: [],
});
it("groups symmetric reverse endpoints and self once, but keeps same-label definitions separate", () => {
  const groups = groupRelationships(
    [
      relation("1", "a", "b"),
      relation("2", "c", "a"),
      relation("3", "a", "a"),
      relation("4", "a", "b", "other"),
    ],
    [definition, { ...definition, id: "other" }],
    "a",
  );
  expect(groups.map((group) => group.relationships.map((r) => r.id))).toEqual([
    ["1", "2", "3"],
    ["4"],
  ]);
});
it("never combines the forward and inverse sides of a directed definition even if their labels match", () => {
  const groups = groupRelationships(
    [relation("1", "a", "b"), relation("2", "c", "a")],
    [{ ...definition, directed: true }],
    "a",
  );
  expect(groups).toHaveLength(2);
});
