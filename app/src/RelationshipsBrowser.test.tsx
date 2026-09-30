import { useState } from "react";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { readProjectRelationships } from "./api";
import { RelationshipsBrowser } from "./RelationshipsBrowser";
import { initialRelationshipView } from "./workspaceHistory";
import type { RelationshipSnapshot } from "./types";

vi.mock("./api", () => ({ readProjectRelationships: vi.fn() }));
const navigate = vi.fn();
function data(): RelationshipSnapshot {
  const person = (id: string) => ({ id, label: id, workspaceState: "active" });
  return {
    globalRevision: 7,
    definitions: [
      {
        id: "ownership",
        name: "Ownership",
        forwardLabel: "owns",
        inverseLabel: "owned by",
        directed: true,
        expectedTargetsPerSource: null,
        expectedSourcesPerTarget: null,
        retired: false,
        revision: 1,
      },
      {
        id: "alliance",
        name: "Alliance",
        forwardLabel: "allied with",
        inverseLabel: "allied with",
        directed: false,
        expectedTargetsPerSource: null,
        expectedSourcesPerTarget: null,
        retired: false,
        revision: 1,
      },
    ],
    relationships: Array.from({ length: 7 }, (_, index) => ({
      id: `relation-${index}`,
      definitionId: index < 5 ? "ownership" : "alliance",
      source: person(index === 6 ? "Captain" : "Navigator"),
      target: person(index === 6 ? "Captain" : `Object ${index}`),
      note: index === 0 ? "An authored note" : "",
      ended: index === 2,
      workspaceState: "active",
      revision: 1,
      warnings: index === 1 ? ["Expected one owner; found two."] : [],
    })),
    entries: [],
  };
}
function Harness({ projectId = "project" }: { projectId?: string }) {
  const [view, setView] = useState(initialRelationshipView);
  return (
    <RelationshipsBrowser
      projectId={projectId}
      view={view}
      onViewChange={setView}
      onNavigate={navigate}
    />
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(readProjectRelationships).mockResolvedValue(data());
});
it("shows five cards initially and reveals the rest without duplicating symmetric or self connections", async () => {
  render(<Harness />);
  await screen.findByText("Showing 5 of 7 relationships");
  expect(screen.getAllByRole("article")).toHaveLength(5);
  expect(screen.queryByRole("table")).not.toBeInTheDocument();
  expect(screen.getByText("Expected one owner; found two.")).toBeVisible();
  fireEvent.click(screen.getByText("Note"));
  expect(screen.getByText("An authored note")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Show more relationships" }));
  expect(screen.getAllByRole("article")).toHaveLength(7);
  expect(screen.getAllByRole("article", { name: "Captain allied with Captain" })).toHaveLength(1);
  expect(screen.queryByRole("button", { name: "Show more relationships" })).not.toBeInTheDocument();
  fireEvent.click(
    within(screen.getAllByRole("article")[0]).getByRole("button", { name: "Object 0" }),
  );
  expect(navigate).toHaveBeenCalledWith("Object 0");
});
it("filters by either participant, combines selected Entries, and intersects definition and state filters", async () => {
  render(<Harness />);
  await screen.findByText("Showing 5 of 7 relationships");
  fireEvent.click(screen.getByText("Entries · All"));
  fireEvent.click(screen.getByRole("checkbox", { name: "Object 2 · active" }));
  expect(screen.getByText("Showing 1 of 1 matching relationships")).toBeVisible();
  expect(screen.getByRole("article")).toHaveAccessibleName("Navigator owns Object 2");
  fireEvent.click(screen.getByRole("checkbox", { name: "Captain · active" }));
  expect(screen.getAllByRole("article")).toHaveLength(2);
  fireEvent.change(screen.getByLabelText("Relationship", { selector: "select" }), {
    target: { value: "alliance" },
  });
  expect(screen.getByRole("article")).toHaveAccessibleName("Captain allied with Captain");
  fireEvent.change(screen.getByLabelText("State"), { target: { value: "ended" } });
  expect(screen.getByText("No relationships match these filters.")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  expect(screen.getAllByRole("article")).toHaveLength(5);
  fireEvent.change(screen.getByLabelText("State"), { target: { value: "current" } });
  expect(screen.getByText("Showing 5 of 6 matching relationships")).toBeVisible();
});
it("distinguishes records by ID when names collide and keeps unavailable participants visible without links", async () => {
  const snapshot = data();
  snapshot.relationships = snapshot.relationships.slice(0, 2);
  snapshot.relationships[0].target = {
    id: null,
    label: "Former object",
    workspaceState: "missing",
  };
  snapshot.relationships[1].target = {
    id: "archived",
    label: "Old object",
    workspaceState: "archived",
  };
  snapshot.definitions[1].name = "Ownership";
  snapshot.relationships[1].definitionId = "alliance";
  vi.mocked(readProjectRelationships).mockResolvedValue(snapshot);
  render(<Harness />);
  await screen.findByText("Showing 2 of 2 relationships");
  expect(screen.getByText("Former object")).toBeVisible();
  expect(
    screen.queryByRole("button", { name: /Former object|Old object/ }),
  ).not.toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Relationship", { selector: "select" }), {
    target: { value: "alliance" },
  });
  expect(screen.getByRole("article")).toHaveAccessibleName("Navigator allied with Old object");
});
it("reports loading failures with a working retry and distinguishes an empty Project", async () => {
  vi.mocked(readProjectRelationships).mockRejectedValueOnce(new Error("Cannot read Project"));
  render(<Harness />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Cannot read Project");
  vi.mocked(readProjectRelationships).mockResolvedValueOnce({
    globalRevision: 0,
    definitions: [],
    entries: [],
    relationships: [],
  });
  fireEvent.click(screen.getByRole("button", { name: "Retry loading relationships" }));
  expect(await screen.findByText(/No relationships yet/)).toBeVisible();
});
it("ignores a late response from a previous Project", async () => {
  let finish!: (data: RelationshipSnapshot) => void;
  vi.mocked(readProjectRelationships).mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  const rendered = render(<Harness projectId="first" />);
  vi.mocked(readProjectRelationships).mockResolvedValueOnce({ ...data(), relationships: [] });
  rendered.rerender(<Harness projectId="second" />);
  await screen.findByText(/No relationships yet/);
  await act(async () => finish(data()));
  expect(screen.queryByRole("article")).not.toBeInTheDocument();
});
