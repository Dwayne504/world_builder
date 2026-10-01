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
  const [recent, setRecent] = useState<string[]>([]);
  return (
    <RelationshipsBrowser
      recentEntryIds={recent}
      onRememberEntry={(id) =>
        setRecent((ids) => [id, ...ids.filter((value) => value !== id)].slice(0, 8))
      }
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
it("opens the exact relationship from Search and lets the reader return to all connections", async () => {
  const change = vi.fn();
  render(
    <RelationshipsBrowser
      projectId="project"
      view={{ ...initialRelationshipView, relationshipId: "relation-6" }}
      onViewChange={change}
      onNavigate={navigate}
    />,
  );
  await screen.findByText("Showing 1 of 1 matching relationships");
  expect(screen.getAllByRole("article")).toHaveLength(1);
  expect(screen.getByRole("article")).toHaveAccessibleName("Captain allied with Captain");
  fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  expect(change).toHaveBeenCalledWith(initialRelationshipView);
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
  fireEvent.change(screen.getByLabelText("Find an Entry"), { target: { value: "Object 2" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Object 2 · active" }));
  expect(screen.getByText("Showing 1 of 1 matching relationships")).toBeVisible();
  expect(screen.getByRole("article")).toHaveAccessibleName("Object 2 owned by Navigator");
  fireEvent.change(screen.getByLabelText("Find an Entry"), { target: { value: "Captain" } });
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

it("bounds suggestions for a thousand Entries and keeps selected and recent choices reachable", async () => {
  const snapshot = data();
  snapshot.entries = Array.from({ length: 1000 }, (_, index) => ({
    id: `entry-${index}`,
    label: `Person ${index}`,
    categoryName: "People",
  }));
  vi.mocked(readProjectRelationships).mockResolvedValue(snapshot);
  render(<Harness />);
  await screen.findByText("Showing 5 of 7 relationships");
  expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  const search = screen.getByLabelText("Find an Entry");
  fireEvent.change(search, { target: { value: "Person" } });
  expect(screen.getAllByRole("checkbox")).toHaveLength(10);
  fireEvent.click(screen.getByRole("button", { name: "Show more Entry results" }));
  expect(screen.getAllByRole("checkbox")).toHaveLength(20);
  for (let index = 100; index < 110; index++) {
    fireEvent.change(search, { target: { value: `Person ${index}` } });
    fireEvent.click(screen.getByRole("checkbox", { name: `Person ${index} · People` }));
  }
  fireEvent.change(search, { target: { value: "" } });
  expect(screen.getAllByRole("checkbox")).toHaveLength(8);
  expect(screen.getAllByRole("checkbox")[0]).toHaveAccessibleName("Person 109 · People");
  fireEvent.click(screen.getByRole("button", { name: "Remove Person 100 filter" }));
  expect(
    screen.queryByRole("button", { name: "Remove Person 100 filter" }),
  ).not.toBeInTheDocument();
  fireEvent.change(search, { target: { value: "No such person" } });
  expect(screen.getByText("No Entries match this search.")).toBeVisible();
});
it("puts a selected symmetric target first without changing the relationship, and uses selection order for ties", async () => {
  const snapshot = data();
  snapshot.relationships = [
    {
      ...snapshot.relationships[5],
      source: { id: "a", label: "Pilot", workspaceState: "active" },
      target: { id: "b", label: "Engineer", workspaceState: "active" },
    },
  ];
  const original = JSON.stringify(snapshot);
  vi.mocked(readProjectRelationships).mockResolvedValue(snapshot);
  render(<Harness />);
  await screen.findByRole("article");
  fireEvent.change(screen.getByLabelText("Find an Entry"), { target: { value: "Engineer" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Engineer · active" }));
  const card = screen.getByRole("article", { name: "Engineer allied with Pilot" });
  expect(within(card).getAllByRole("button")[0]).toHaveAccessibleName("Engineer");
  fireEvent.change(screen.getByLabelText("Find an Entry"), { target: { value: "Pilot" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Pilot · active" }));
  expect(card).toHaveAccessibleName("Engineer allied with Pilot");
  expect(JSON.stringify(snapshot)).toBe(original);
});
