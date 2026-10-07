import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { CategoryManager } from "./CategoryManager";
import {
  applyStructure,
  previewCategoryDelete,
  applyTemplateFields,
  applySpatial,
  readSpatial,
  createCategory,
  createType,
  listCategories,
  listTypes,
  readFieldCatalog,
  readProjectRelationships,
  getPreferences,
  previewFieldMerge,
  mergeFields,
} from "./api";
import type { FieldsController } from "./EntryFieldsPanel";
import type { FieldCatalog, FieldDefinition } from "./types";
vi.mock("./api", () => ({
  applyStructure: vi.fn(),
  previewCategoryDelete: vi.fn(),
  readSpatial: vi.fn().mockResolvedValue({ globalRevision: 1, entries: [], defaults: [] }),
  applySpatial: vi.fn(),
  getPreferences: vi.fn(),
  previewFieldMerge: vi.fn(),
  mergeFields: vi.fn(),
  pickDirectory: vi.fn(),
  applyTemplateFields: vi.fn(),
  createCategory: vi.fn(),
  createType: vi.fn(),
  listCategories: vi.fn(),
  listTypes: vi.fn(),
  readFieldCatalog: vi.fn(),
  readProjectRelationships: vi.fn(),
}));
const weapons = {
  id: "weapons",
  name: "Weapons",
  isUncategorized: false,
  revision: 1,
  globalRevision: 1,
};
const sword = {
  id: "sword",
  categoryId: "weapons",
  parentTypeId: null,
  name: "Sword",
  revision: 1,
  globalRevision: 1,
};
const mass: FieldDefinition = {
  id: "mass",
  name: "Mass",
  kind: "number",
  unit: "tons",
  revision: 1,
  retired: false,
  options: [],
  bindings: [],
};
let controller: FieldsController;
const changed = vi.fn();
const close = vi.fn();
const props = {
  projectId: "project",
  open: true,
  onClose: close,
  onChanged: changed,
  onController: (value: FieldsController) => {
    controller = value;
  },
};
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(applyStructure).mockReset();
  vi.mocked(previewCategoryDelete).mockReset();
  vi.mocked(listCategories).mockResolvedValue([weapons]);
  vi.mocked(listTypes).mockImplementation((_project, categoryId) =>
    Promise.resolve(categoryId === weapons.id ? [sword] : []),
  );
  vi.mocked(readFieldCatalog).mockResolvedValue({ globalRevision: 1, definitions: [{ ...mass }] });
  vi.mocked(applyTemplateFields).mockResolvedValue({ globalRevision: 2, definitions: [] });
});

it("opens the requested Category and preserves a dismissed rename as a rename", async () => {
  const places = { ...weapons, id: "places", name: "Places" };
  vi.mocked(listCategories).mockResolvedValue([weapons, places]);
  vi.mocked(applyStructure).mockResolvedValue({ globalRevision: 2, backupPath: null });
  render(<CategoryManager {...props} initialCategoryId="places" />);
  await screen.findByText("Types in Places");
  fireEvent.click(screen.getByText("Category actions"));
  fireEvent.click(screen.getByRole("button", { name: "Rename Category" }));
  change("New Category name", "");
  fireEvent.click(screen.getByRole("button", { name: "Close Rename Category" }));
  fireEvent.click(screen.getByRole("button", { name: "Continue manager draft" }));
  expect(screen.getByLabelText("New Category name")).toHaveValue("");
  expect(screen.getByRole("button", { name: "Save Category name" })).toBeDisabled();
  change("New Category name", "Locations");
  fireEvent.click(screen.getByRole("button", { name: "Save Category name" }));
  await waitFor(() =>
    expect(applyStructure).toHaveBeenCalledWith("project", 1, {
      kind: "rename_category",
      id: "places",
      name: "Locations",
    }),
  );
  await waitFor(() => expect(controller.state).toBe("saved"));
  expect(createCategory).not.toHaveBeenCalled();
});

it("requires reassignment and explicit Type review, retains failures, and waits for acknowledged deletion", async () => {
  const fallback = { ...weapons, id: "fallback", name: "Uncategorized", isUncategorized: true };
  vi.mocked(listCategories).mockResolvedValue([weapons, fallback]);
  vi.mocked(previewCategoryDelete).mockResolvedValue({
    globalRevision: 1,
    name: "Weapons",
    entryCount: 3,
    typedEntryCount: 2,
    typeNames: ["Sword"],
    defaultCount: 1,
  });
  vi.mocked(applyStructure).mockRejectedValueOnce(new Error("Recovery copy unavailable"));
  const deleted = vi.fn();
  const backup = vi.fn();
  render(<CategoryManager {...props} onDeleted={deleted} onRecoveryBackup={backup} />);
  await screen.findByText("Types in Weapons");
  fireEvent.click(screen.getByText("Category actions"));
  fireEvent.click(screen.getByRole("button", { name: "Delete Category…" }));
  const dialog = screen.getByRole("dialog", { name: "Delete Category" });
  const confirm = await within(dialog).findByRole("button", { name: "Delete Category" });
  expect(confirm).toBeDisabled();
  change("Move Entries to", "fallback");
  expect(confirm).toBeDisabled();
  fireEvent.click(within(dialog).getByRole("checkbox"));
  fireEvent.click(confirm);
  await within(dialog).findByText(/Recovery copy unavailable/);
  expect(deleted).not.toHaveBeenCalled();
  let finish!: (value: { globalRevision: number; backupPath: string }) => void;
  vi.mocked(applyStructure).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  fireEvent.click(confirm);
  expect(controller.state).toBe("saving");
  expect(confirm).toBeDisabled();
  await act(async () => {
    finish({ globalRevision: 2, backupPath: "/synthetic/recovery.wcbackup" });
    expect(await controller.submit()).toEqual({ kind: "committed" });
  });
  expect(applyStructure).toHaveBeenLastCalledWith("project", 1, {
    kind: "delete_category",
    id: "weapons",
    destinationId: "fallback",
    removeTypes: true,
  });
  expect(deleted).toHaveBeenCalledTimes(1);
  expect(backup).toHaveBeenCalledWith("/synthetic/recovery.wcbackup");
});

it("protects Uncategorized from deletion and rename", async () => {
  vi.mocked(listCategories).mockResolvedValue([
    { ...weapons, isUncategorized: true, name: "Uncategorized" },
  ]);
  render(<CategoryManager {...props} />);
  await screen.findByText("Types in Uncategorized");
  expect(screen.queryByRole("button", { name: "Delete Category…" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Rename Category" })).not.toBeInTheDocument();
});
async function show() {
  const view = render(<CategoryManager {...props} />);
  await screen.findByText("Types in Weapons");
  return view;
}
it("configures a relationship Field as a Type default without storing a separate value", async () => {
  vi.mocked(readProjectRelationships).mockResolvedValue({
    globalRevision: 1,
    entries: [],
    relationships: [],
    definitions: [
      {
        id: "ownership",
        name: "Ownership",
        forwardLabel: "owns",
        inverseLabel: "is owned by",
        directed: true,
        expectedSourcesPerTarget: 1,
        expectedTargetsPerSource: null,
        revision: 1,
        retired: false,
      },
    ],
  });
  await show();
  fireEvent.change(screen.getByLabelText("Default field scope"), { target: { value: "sword" } });
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Current owner");
  change("Default field kind", "relationship");
  expect(screen.getByRole("button", { name: "Create default field" })).toBeDisabled();
  await screen.findByRole("option", { name: "Ownership · owns / is owned by" });
  change("Field relationship", "ownership");
  change("Field relationship direction", "target");
  fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledWith("project", 1, {
      kind: "create_projection",
      name: "Current owner",
      relationshipDefinitionId: "ownership",
      perspective: "target",
      provider: { kind: "type", id: "sword" },
    }),
  );
  await waitFor(() => expect(controller.state).toBe("saved"));
});
function change(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}
it("creates optional numeric defaults directly on a Category without an Entry", async () => {
  await show();
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Mass");
  change("Default field kind", "number");
  change("Default field unit", "tons");
  fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledWith("project", 1, {
      kind: "create",
      name: "Mass",
      fieldKind: "number",
      unit: "tons",
      provider: { kind: "category", id: "weapons" },
      options: [],
      value: null,
    }),
  );
  await waitFor(() => expect(controller.state).toBe("saved"));
  expect(changed).toHaveBeenCalledWith(2);
});
it("lists Types and configures Type defaults by their identity", async () => {
  await show();
  fireEvent.click(screen.getByRole("button", { name: "Defaults for Sword" }));
  fireEvent.click(screen.getByRole("button", { name: "Reuse field" }));
  change("Existing default field", "mass");
  fireEvent.click(screen.getByRole("button", { name: "Use as default" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledWith("project", 1, {
      kind: "bind",
      fieldId: "mass",
      provider: { kind: "type", id: "sword" },
    }),
  );
});
it("removes only the availability binding and explains retained values", async () => {
  vi.mocked(readFieldCatalog).mockResolvedValue({
    globalRevision: 4,
    definitions: [
      { ...mass, bindings: [{ provider: { kind: "category", id: "weapons" }, label: "Weapons" }] },
    ],
  });
  await show();
  expect(screen.getByText(/Removing a default preserves values/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Remove default: Mass" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledWith("project", 4, {
      kind: "unbind",
      fieldId: "mass",
      provider: { kind: "category", id: "weapons" },
    }),
  );
});
it("keeps hidden and failed drafts and blocks close until explicitly cancelled", async () => {
  vi.mocked(applyTemplateFields).mockRejectedValueOnce(new Error("Stale revision"));
  const view = await show();
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Range");
  fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
  await screen.findByText("Stale revision");
  await waitFor(() => expect(controller.state).toBe("failed"));
  view.rerender(<CategoryManager {...props} open={false} />);
  expect(screen.getByText(/unfinished draft/)).toBeVisible();
  await act(async () => {
    expect(await controller.submit()).toEqual({ kind: "failed" });
  });
  view.rerender(<CategoryManager {...props} />);
  await waitFor(() => expect(controller.state).toBe("dirty"));
  expect(screen.getByLabelText("Default field name")).toHaveValue("Range");
  fireEvent.click(screen.getByRole("button", { name: "Cancel manager draft" }));
  expect(controller.canSubmit).toBe(true);
});
it("waits for acknowledged creation and clears only committed work", async () => {
  let resolve!: (catalog: FieldCatalog) => void;
  vi.mocked(applyTemplateFields).mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  await show();
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Range");
  fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
  await waitFor(() => expect(controller.state).toBe("saving"));
  let finished = false;
  let waiting: Promise<unknown>;
  act(() => {
    waiting = controller.submit().then(() => {
      finished = true;
    });
  });
  expect(finished).toBe(false);
  await act(async () => {
    resolve({ globalRevision: 2, definitions: [] });
    await waiting;
  });
  expect(finished).toBe(true);
  expect(screen.getByLabelText("Default field name")).toHaveValue("");
});
it("creates Categories and Types and refreshes the manager in the same session", async () => {
  vi.mocked(createType).mockResolvedValue({ ...sword, id: "axe", name: "Axe", globalRevision: 3 });
  await show();
  fireEvent.click(screen.getByRole("button", { name: "Add Type" }));
  change("Category manager Type name", "Axe");
  vi.mocked(listTypes).mockResolvedValue([sword, { ...sword, id: "axe", name: "Axe" }]);
  fireEvent.click(screen.getByRole("button", { name: "Create Type" }));
  await screen.findByRole("button", { name: "Defaults for Axe" });
  expect(createType).toHaveBeenCalledWith("project", "weapons", "Axe", undefined);
  vi.mocked(createCategory).mockResolvedValue({
    ...weapons,
    id: "places",
    name: "Places",
    globalRevision: 4,
  });
  vi.mocked(listCategories).mockResolvedValue([
    weapons,
    { ...weapons, id: "places", name: "Places" },
  ]);
  fireEvent.click(screen.getByRole("button", { name: "Add Category" }));
  change("Category manager name", "Places");
  fireEvent.click(screen.getByRole("button", { name: "Create Category" }));
  await screen.findByText("Types in Places");
  expect(changed).toHaveBeenLastCalledWith(4);
});

it("uses creation dialogs and retains a child draft when returning to the manager", async () => {
  await show();
  expect(screen.getByLabelText("Default field name")).not.toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Reach");
  fireEvent(
    screen.getByRole("dialog", { name: "Add default field" }),
    new Event("cancel", { cancelable: true }),
  );
  expect(screen.getByRole("dialog", { name: "Categories and defaults" })).toBeVisible();
  expect(screen.getByRole("button", { name: "Continue manager draft" })).toBeVisible();
  expect(controller.canSubmit).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Continue manager draft" }));
  expect(screen.getByLabelText("Default field name")).toHaveValue("Reach");
  fireEvent.click(screen.getByRole("button", { name: "Cancel manager draft" }));
  expect(controller.state).toBe("saved");
  expect(applyTemplateFields).not.toHaveBeenCalled();
});

it("reuses an existing local definition for Type defaults instead of copying values", async () => {
  await show();
  fireEvent.click(screen.getByRole("button", { name: "Defaults for Sword" }));
  fireEvent.click(screen.getByRole("button", { name: "Add default field" }));
  change("Default field name", "Mass");
  fireEvent.click(screen.getByRole("button", { name: "Reuse Mass" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledWith("project", 1, {
      kind: "bind",
      fieldId: "mass",
      provider: { kind: "type", id: "sword" },
    }),
  );
});

it("tracks a merge until it commits and never offers an acknowledged merge as a failed write", async () => {
  const duplicate = { ...mass, id: "duplicate" };
  vi.mocked(readFieldCatalog).mockResolvedValue({
    globalRevision: 1,
    definitions: [mass, duplicate],
  });
  vi.mocked(getPreferences).mockResolvedValue({
    defaultProjectsDir: null,
    defaultProjectsDirExists: false,
    defaultBackupsDir: "/Backups",
    defaultBackupsDirExists: true,
  });
  vi.mocked(previewFieldMerge).mockResolvedValue({
    globalRevision: 1,
    source: duplicate,
    target: mass,
    entries: [],
    blockers: [],
  });
  let finish!: (result: { globalRevision: number; backupPath: string }) => void;
  vi.mocked(mergeFields).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  await show();
  fireEvent.click(screen.getByText("Field maintenance"));
  fireEvent.click(screen.getByRole("button", { name: "Combine duplicate fields" }));
  change("Keep Field", "mass");
  change("Duplicate Field", "duplicate");
  fireEvent.click(screen.getByRole("button", { name: "Review merge" }));
  await screen.findByText("Review: Mass");
  fireEvent.click(
    screen.getByRole("checkbox", { name: "I reviewed the values and combined defaults." }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Back up and merge" }));
  expect(controller.state).toBe("saving");
  vi.mocked(readFieldCatalog).mockRejectedValueOnce(new Error("Refresh unavailable"));
  await act(async () => {
    finish({ globalRevision: 2, backupPath: "/Backups/before.wcbackup" });
    expect(await controller.submit()).toEqual({ kind: "committed" });
  });
  await screen.findByText(/Refresh unavailable/);
  expect(screen.getByRole("status")).toHaveTextContent("Fields combined.");
  expect(changed).toHaveBeenCalledWith(2);
  expect(screen.queryByRole("button", { name: "Back up and merge" })).not.toBeInTheDocument();
});

it("saves Spatial defaults for the selected Type without applying them to existing Entries", async () => {
  await show();
  fireEvent.change(screen.getByLabelText("Default field scope"), { target: { value: "sword" } });
  vi.mocked(readSpatial).mockResolvedValueOnce({
    globalRevision: 2,
    entries: [],
    defaults: [{ kind: "type", id: "sword" }],
  });
  vi.mocked(applySpatial).mockResolvedValueOnce({
    globalRevision: 2,
    entries: [],
    defaults: [{ kind: "type", id: "sword" }],
  });
  fireEvent.click(screen.getByRole("checkbox", { name: "Spatial — can contain other places" }));
  await waitFor(() =>
    expect(applySpatial).toHaveBeenCalledWith("project", 1, {
      kind: "set_default",
      provider: { kind: "type", id: "sword" },
      enabled: true,
    }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("checkbox", { name: "Spatial — can contain other places" }),
    ).toBeChecked(),
  );
  expect(screen.getByText(/Existing Entries keep their features/)).toBeInTheDocument();
  expect(changed).toHaveBeenCalledWith(2);
});

it("finds a Type beyond one hundred items and keeps its selected defaults when the list is filtered", async () => {
  vi.mocked(listTypes).mockResolvedValue(
    Array.from({ length: 125 }, (_, index) => ({
      ...sword,
      id: `type-${index}`,
      name: `Type ${index}`,
    })),
  );
  await show();
  expect(screen.getAllByRole("button", { name: /^Defaults for Type/ })).toHaveLength(12);
  change("Find a Type", "Type 124");
  fireEvent.click(screen.getByRole("button", { name: "Defaults for Type 124" }));
  expect(screen.getByRole("button", { name: "Defaults for Type 124" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  expect(screen.getByLabelText("Default field scope")).toHaveValue("type-124");
  change("Find a Type", "No matching Type");
  expect(screen.getByText("No Types match your search.")).toBeVisible();
  expect(screen.getByLabelText("Default field scope")).toHaveValue("type-124");
  expect(screen.getByText(/Editing defaults for/)).toHaveTextContent("Type 124");
  expect(applyTemplateFields).not.toHaveBeenCalled();
});

it("searches a hundred defaults and removes only the chosen binding", async () => {
  const definitions = Array.from({ length: 110 }, (_, index) => ({
    ...mass,
    id: `field-${index}`,
    name: `Measure ${index}`,
    bindings: [{ provider: { kind: "category" as const, id: "weapons" }, label: "Weapons" }],
  }));
  vi.mocked(readFieldCatalog).mockResolvedValue({ globalRevision: 1, definitions });
  await show();
  expect(screen.getAllByRole("button", { name: /^Remove default:/ })).toHaveLength(12);
  change("Find a default field", "Measure 109");
  fireEvent.click(screen.getByRole("button", { name: "Remove default: Measure 109" }));
  await waitFor(() =>
    expect(applyTemplateFields).toHaveBeenCalledExactlyOnceWith("project", 1, {
      kind: "unbind",
      fieldId: "field-109",
      provider: { kind: "category", id: "weapons" },
    }),
  );
});
