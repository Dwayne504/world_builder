import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { RelationshipGroup } from "./RelationshipGroup";
import type { Relationship } from "./types";

it("bounds a large group, preserves every warning, and keeps a restored target reachable", () => {
  const relationships: Relationship[] = Array.from({ length: 125 }, (_, index) => ({
    id: `relation-${index}`,
    definitionId: "alliance",
    source: { id: "source", label: "Navigator", workspaceState: "active" },
    target: { id: `person-${index}`, label: `Person ${index}`, workspaceState: "active" },
    note: "",
    workspaceState: "active",
    ended: false,
    revision: 1,
    warnings: index === 124 ? ["Expected one ally; found several."] : [],
  }));
  render(
    <ul>
      <RelationshipGroup
        label="allied with"
        entryId="source"
        relationships={relationships}
        restoreRelationshipId="relation-124"
        initialOpen
        connection={(relation) => (
          <li key={relation.id}>
            <button>{relation.target.label}</button>
          </li>
        )}
      />
    </ul>,
  );
  expect(screen.getByRole("button", { name: "Person 124" })).toBeVisible();
  expect(screen.getAllByRole("button", { name: /^Person/ })).toHaveLength(12);
  expect(screen.getByText("and 122 more")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Hide allied with relationships" }));
  expect(screen.getByText("Expected one ally; found several.")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Show allied with relationships" }));
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Person 123" } });
  expect(screen.getAllByRole("button", { name: /^Person/ })).toHaveLength(1);
  expect(screen.getByRole("button", { name: "Person 123" })).toBeVisible();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Show more allied with connections" }));
  expect(screen.getAllByRole("button", { name: /^Person/ })).toHaveLength(24);
  expect(screen.getByRole("button", { name: "Person 124" })).toBeVisible();
});
