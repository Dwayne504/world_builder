import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { EntryFieldsPanel, type FieldsController } from "./EntryFieldsPanel";
import {
  applyFields,
  deleteEntryField,
  listCategories,
  readFields,
  readProjectRelationships,
} from "./api";
import type {
  Entry,
  EntryFields,
  FieldDefinition,
  Relationship,
  RelationshipSnapshot,
} from "./types";

vi.mock("./api", () => ({
  applyFields: vi.fn(),
  deleteEntryField: vi.fn(),
  listCategories: vi.fn(),
  readFields: vi.fn(),
  readProjectRelationships: vi.fn(),
}));
const entry: Entry = {
  id: "blade",
  categoryId: "weapons",
  typeId: null,
  authoredName: "Blade",
  displayName: "Blade",
  revision: 1,
  globalRevision: 5,
};
const definition: FieldDefinition = {
  id: "owner-field",
  name: "Current owner",
  kind: "relationship",
  projection: { relationshipDefinitionId: "ownership", perspective: "target" },
  retired: false,
  revision: 1,
  bindings: [],
  options: [],
};
const connection: Relationship = {
  id: "connection",
  definitionId: "ownership",
  source: { id: "owner", label: "Robin", workspaceState: "active" },
  target: { id: entry.id, label: entry.displayName, workspaceState: "active" },
  ended: false,
  workspaceState: "active",
  revision: 2,
  note: "An inherited blade",
  warnings: [],
};
const snapshot: EntryFields = {
  globalRevision: 5,
  definitions: [definition],
  fields: [{ definition, available: true, value: null, projectedRelationships: [connection] }],
};
const relationships: RelationshipSnapshot = {
  globalRevision: 5,
  definitions: [
    {
      id: "ownership",
      name: "Ownership",
      forwardLabel: "owns",
      inverseLabel: "is owned by",
      directed: true,
      expectedTargetsPerSource: null,
      expectedSourcesPerTarget: 1,
      retired: false,
      revision: 1,
    },
  ],
  relationships: [connection],
  entries: [{ id: "new-owner", label: "Dana", categoryName: "Characters" }],
};
let controller: FieldsController;
const presented = vi.fn();
const entriesChanged = vi.fn();
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readFields).mockResolvedValue(snapshot);
  vi.mocked(applyFields).mockResolvedValue({ ...snapshot, globalRevision: 6 });
  vi.mocked(readProjectRelationships).mockResolvedValue(relationships);
  vi.mocked(listCategories).mockResolvedValue([
    { id: "places", name: "Places", isUncategorized: false, revision: 1, globalRevision: 5 },
  ]);
});
function show() {
  render(
    <EntryFieldsPanel
      projectId="project"
      entry={entry}
      disabled={false}
      onController={(value) => {
        controller = value;
      }}
      onRevision={vi.fn()}
      onPresentedRelationships={presented}
      onEntriesChanged={entriesChanged}
    />,
  );
}

it("shows canonical notes, warnings and inactive participants without a scalar input", async () => {
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    fields: [
      {
        ...snapshot.fields[0],
        projectedRelationships: [
          {
            ...connection,
            source: { ...connection.source, workspaceState: "archived" },
            warnings: ["Expected one; found 2"],
          },
        ],
      },
    ],
  });
  show();
  expect(await screen.findByText("Robin")).toBeVisible();
  expect(screen.getByText("archived")).toBeVisible();
  expect(screen.getByText("Expected one; found 2")).toBeVisible();
  fireEvent.click(screen.getByText("Note"));
  expect(screen.getByText("An inherited blade")).toBeVisible();
  expect(screen.queryByLabelText("Value: Current owner")).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Clear value: Current owner")).not.toBeInTheDocument();
  expect(presented).toHaveBeenLastCalledWith(["connection"]);
});

it("retains a detached projection visibly and requires restoring its availability before editing", async () => {
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    fields: [{ ...snapshot.fields[0], available: false }],
  });
  show();
  expect(await screen.findByText("Robin")).toBeVisible();
  expect(screen.getByRole("button", { name: "Change Current owner: Robin" })).toBeDisabled();
  expect(
    screen.getByText("Restore this Field in Manage fields before changing its connection."),
  ).toBeVisible();
});

it("changes the explicitly selected conflicting instance and keeps failed drafts for retry", async () => {
  const second = {
    ...connection,
    id: "second",
    source: { ...connection.source, id: "second-owner", label: "Alex" },
  };
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    fields: [{ ...snapshot.fields[0], projectedRelationships: [connection, second] }],
  });
  vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Change Current owner: Alex" }));
  expect(controller).toMatchObject({ state: "dirty", canSubmit: false });
  fireEvent.change(screen.getByLabelText("Find Entry for Current owner"), {
    target: { value: "Dana" },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Dana Characters" }));
  await screen.findByText("Disk full");
  expect(screen.getByLabelText("Find Entry for Current owner")).toHaveValue("Dana");
  expect(controller).toMatchObject({ state: "failed", canSubmit: false });
  expect(applyFields).toHaveBeenCalledWith("project", "blade", 5, {
    kind: "edit_projection",
    fieldId: "owner-field",
    instanceId: "second",
    other: { kind: "existing", id: "new-owner" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Dana Characters" }));
  await waitFor(() => expect(controller).toMatchObject({ state: "saved", canSubmit: true }));
  expect(screen.queryByLabelText("Find Entry for Current owner")).not.toBeInTheDocument();
  expect(entriesChanged).not.toHaveBeenCalled();
});

it("creates and connects a stub inline from an empty projection", async () => {
  vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    fields: [{ ...snapshot.fields[0], projectedRelationships: [] }],
  });
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Choose Current owner" }));
  fireEvent.change(screen.getByLabelText("Find Entry for Current owner"), {
    target: { value: "Kharon" },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Create “Kharon”" }));
  fireEvent.change(screen.getByLabelText("New connected Entry Category"), {
    target: { value: "places" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Create and connect" }));
  await screen.findByText("Disk full");
  expect(entriesChanged).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Create and connect" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith("project", "blade", 5, {
      kind: "edit_projection",
      fieldId: "owner-field",
      instanceId: null,
      other: { kind: "create", name: "Kharon", categoryId: "places" },
    }),
  );
  await waitFor(() => expect(controller.state).toBe("saved"));
  expect(entriesChanged).toHaveBeenCalledTimes(1);
});

it("limits search results and allows cancelling without a mutation", async () => {
  vi.mocked(readProjectRelationships).mockResolvedValue({
    ...relationships,
    entries: Array.from({ length: 20 }, (_, index) => ({
      id: String(index),
      label: `Person ${index}`,
      categoryName: "Characters",
    })),
  });
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Change Current owner: Robin" }));
  fireEvent.change(screen.getByLabelText("Find Entry for Current owner"), {
    target: { value: "Person" },
  });
  await screen.findByRole("button", { name: "Show more matches (10 of 20)" });
  expect(screen.getAllByRole("button", { name: /Person \d+ Characters/ })).toHaveLength(10);
  fireEvent.click(screen.getByRole("button", { name: "Show more matches (10 of 20)" }));
  expect(screen.getAllByRole("button", { name: /Person \d+ Characters/ })).toHaveLength(20);
  fireEvent.click(screen.getByRole("button", { name: "Cancel connection change" }));
  expect(controller).toMatchObject({ state: "saved", canSubmit: true });
  expect(applyFields).not.toHaveBeenCalled();
});

it("creates a target-perspective Field through the relationship command", async () => {
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Add field" }));
  fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Owner" } });
  fireEvent.change(screen.getByLabelText("new-field-kind"), { target: { value: "relationship" } });
  expect(screen.getByRole("button", { name: "Create field" })).toBeDisabled();
  await screen.findByRole("option", { name: "Ownership · owns / is owned by" });
  fireEvent.change(screen.getByLabelText("Field relationship"), { target: { value: "ownership" } });
  fireEvent.change(screen.getByLabelText("Field relationship direction"), {
    target: { value: "target" },
  });
  expect(screen.queryByLabelText("new-field-value")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Create field" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith("project", "blade", 5, {
      kind: "create_projection",
      name: "Owner",
      relationshipDefinitionId: "ownership",
      perspective: "target",
      provider: { kind: "entry", id: "blade" },
    }),
  );
});

it("removes only the Field display and reports only visible relationships", async () => {
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    fields: [{ ...snapshot.fields[0], hidden: true }],
  });
  show();
  await screen.findByRole("button", { name: "Show hidden fields (1)" });
  expect(presented).toHaveBeenLastCalledWith([]);
  fireEvent.click(screen.getByRole("button", { name: "Show hidden fields (1)" }));
  expect(presented).toHaveBeenLastCalledWith(["connection"]);
  fireEvent.click(screen.getByRole("button", { name: "Manage fields" }));
  const manager = within(screen.getByRole("dialog", { name: "Manage fields" }));
  expect(
    manager.queryByRole("button", { name: "Delete from Entry: Current owner" }),
  ).not.toBeInTheDocument();
  fireEvent.click(manager.getByRole("button", { name: "Remove from Fields: Current owner" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith("project", "blade", 5, {
      kind: "remove_projection",
      fieldId: "owner-field",
    }),
  );
  expect(deleteEntryField).not.toHaveBeenCalled();
  await act(async () => {});
});
