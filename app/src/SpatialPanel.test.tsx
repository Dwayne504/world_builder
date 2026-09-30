import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { SpatialPanel } from "./SpatialPanel";
import { applySpatial, readSpatial } from "./api";
import type { SpatialSnapshot } from "./types";
import type { FieldsController } from "./EntryFieldsPanel";
import { spatialPath, spatialDescendants } from "./spatialPresentation";

vi.mock("./api", () => ({
  applySpatial: vi.fn(),
  readSpatial: vi.fn(),
  readRelationships: vi
    .fn()
    .mockResolvedValue({ globalRevision: 8, entries: [], relationships: [], definitions: [] }),
}));
const entry = (id: string, parentId: string | null = null) => ({
  id,
  label: id,
  parentId,
  spatial: true,
  workspaceState: "active",
});
const snapshot: SpatialSnapshot = {
  globalRevision: 8,
  defaults: [],
  entries: [
    entry("Tortuga"),
    entry("Northern Shell", "Tortuga"),
    entry("Arak", "Northern Shell"),
    entry("Temple", "Arak"),
    entry("Floating Continent"),
    { ...entry("Thron"), spatial: false },
  ],
};
let controller: FieldsController;
const onRevision = vi.fn();
const onNavigate = vi.fn();
const onEntriesChanged = vi.fn();
function mount(entryId = "Arak", relations?: import("./types").RelationshipSnapshot) {
  return render(
    <SpatialPanel
      projectId="world"
      relations={relations}
      entryId={entryId}
      categories={[]}
      disabled={false}
      onController={(next) => {
        controller = next;
      }}
      onRevision={onRevision}
      getRevision={() => 9}
      onNavigate={onNavigate}
      onEntriesChanged={onEntriesChanged}
      onCommitted={vi.fn()}
      refreshKey={0}
    />,
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(readSpatial).mockResolvedValue(structuredClone(snapshot));
});

it("shows an explicitly labelled derived path for a direct relationship without turning it into ancestor relationships", async () => {
  const relations: import("./types").RelationshipSnapshot = {
    globalRevision: 8,
    entries: [],
    definitions: [
      {
        id: "location",
        name: "Current Location",
        forwardLabel: "is at",
        inverseLabel: "hosts",
        directed: true,
        expectedTargetsPerSource: 1,
        expectedSourcesPerTarget: null,
        retired: false,
        revision: 1,
      },
    ],
    relationships: [
      {
        id: "visit",
        definitionId: "location",
        source: { id: "Thron", label: "Thron", workspaceState: "active" },
        target: { id: "Temple", label: "Temple", workspaceState: "active" },
        note: "",
        ended: false,
        workspaceState: "active",
        revision: 1,
        warnings: [],
      },
    ],
  };
  mount("Thron", relations);
  fireEvent.click(await screen.findByText("Related places"));
  expect(screen.getByText("is at")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Arak" }));
  expect(onNavigate).toHaveBeenCalledWith("Arak");
  expect(applySpatial).not.toHaveBeenCalled();
});

it("derives breadcrumbs and descendants from parent links, including safe handling of a corrupt cycle", () => {
  expect(spatialPath(snapshot.entries, "Temple").map((e) => e.id)).toEqual([
    "Tortuga",
    "Northern Shell",
    "Arak",
    "Temple",
  ]);
  expect([...spatialDescendants(snapshot.entries, "Tortuga")]).toEqual([
    "Northern Shell",
    "Arak",
    "Temple",
  ]);
  expect(spatialPath([entry("A", "B"), entry("B", "A")], "A")).toHaveLength(2);
});

it("shows direct children, searchable descendants and navigable breadcrumbs", async () => {
  mount("Tortuga");
  await screen.findByRole("button", { name: "Northern Shell" });
  expect(screen.queryByRole("button", { name: "Temple" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Contents"), { target: { value: "all" } });
  fireEvent.change(screen.getByLabelText("Find a place inside"), { target: { value: "Temple" } });
  fireEvent.click(screen.getByRole("button", { name: "Temple" }));
  expect(onNavigate).toHaveBeenCalledWith("Temple");
  expect(screen.queryByRole("button", { name: "Arak" })).not.toBeInTheDocument();
});

it("previews legal parents, keeps a failed move visible and updates breadcrumbs after retry", async () => {
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Arrange places" }));
  fireEvent.click(screen.getByRole("button", { name: "Change parent" }));
  const dialog = screen.getByRole("dialog");
  expect(
    within(dialog).queryByRole("button", { name: "Move inside Temple" }),
  ).not.toBeInTheDocument();
  expect(
    within(dialog).queryByRole("button", { name: "Move inside Arak" }),
  ).not.toBeInTheDocument();
  expect(
    within(dialog).queryByRole("button", { name: "Move inside Thron" }),
  ).not.toBeInTheDocument();
  vi.mocked(applySpatial).mockRejectedValueOnce(new Error("Disk unavailable"));
  fireEvent.click(screen.getByRole("button", { name: "Move inside Floating Continent" }));
  await screen.findByRole("alert");
  expect(controller.state).toBe("failed");
  expect(await controller.submit()).toEqual({ kind: "failed" });
  expect(applySpatial).toHaveBeenCalledWith("world", 9, {
    kind: "reparent",
    entryId: "Arak",
    parentId: "Floating Continent",
  });
  vi.mocked(applySpatial).mockResolvedValue({
    ...snapshot,
    globalRevision: 10,
    entries: snapshot.entries.map((e) =>
      e.id === "Arak" ? { ...e, parentId: "Floating Continent" } : e,
    ),
  });
  fireEvent.click(screen.getByRole("button", { name: "Move inside Floating Continent" }));
  await waitFor(() => expect(controller.state).toBe("saved"));
  expect(onRevision).toHaveBeenCalledWith(10);
  expect(screen.getByRole("navigation", { name: "Spatial breadcrumbs" })).toHaveTextContent(
    "Floating Continent › Arak",
  );
});

it("retains a dismissed child draft and waits for the creation acknowledgement", async () => {
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Arrange places" }));
  fireEvent.click(screen.getByRole("button", { name: "Create child" }));
  fireEvent.change(screen.getByLabelText("Child name (optional)"), {
    target: { value: "Observatory" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Close Arrange places" }));
  expect(controller.state).toBe("dirty");
  expect(controller.canSubmit).toBe(false);
  expect(await controller.submit()).toEqual({ kind: "failed" });
  fireEvent.click(screen.getByRole("button", { name: "Continue Spatial draft" }));
  expect(screen.getByLabelText("Child name (optional)")).toHaveValue("Observatory");
  let resolve!: (next: SpatialSnapshot) => void;
  vi.mocked(applySpatial).mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Create place inside Arak" }));
  expect(controller.state).toBe("saving");
  const waiting = controller.submit();
  fireEvent.click(screen.getByRole("button", { name: "Close Arrange places" }));
  expect(screen.getByRole("dialog")).toBeInTheDocument();
  await act(async () => {
    resolve({
      ...snapshot,
      globalRevision: 10,
      entries: [...snapshot.entries, entry("Observatory", "Arak")],
    });
    await waiting;
  });
  expect(controller.state).toBe("saved");
  expect(onEntriesChanged).toHaveBeenCalledOnce();
  expect(screen.getByRole("button", { name: "Observatory" })).toBeInTheDocument();
});

it("enables Spatial explicitly without asking for reclassification and blocks removal with dependents", async () => {
  mount("Thron");
  await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
  fireEvent.click(screen.getByText("Spatial feature"));
  vi.mocked(applySpatial).mockResolvedValue({
    ...snapshot,
    globalRevision: 10,
    entries: snapshot.entries.map((e) => (e.id === "Thron" ? { ...e, spatial: true } : e)),
  });
  fireEvent.click(screen.getByRole("button", { name: "Enable Spatial" }));
  await screen.findByRole("button", { name: "Arrange places" });
  expect(applySpatial).toHaveBeenCalledWith("world", 9, {
    kind: "set_enabled",
    entryId: "Thron",
    enabled: true,
  });
});

it("disables removal for an Entry with a parent or children and bounds long place lists", async () => {
  vi.mocked(readSpatial).mockResolvedValue({
    ...snapshot,
    entries: [
      ...snapshot.entries,
      ...Array.from({ length: 30 }, (_, i) => entry(`Place ${i}`, "Arak")),
    ],
  });
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Arrange places" }));
  fireEvent.click(screen.getByText("Remove Spatial feature"));
  expect(screen.getByRole("button", { name: "Remove Spatial" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Close Arrange places" }));
  fireEvent.click(screen.getByRole("button", { name: "Show more places (11 remaining)" }));
  expect(screen.getByRole("button", { name: "Place 29" })).toBeInTheDocument();
});
