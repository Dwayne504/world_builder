import { menuItem, renderWithMenu as render } from "./desktopMenuTestUtils";
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EntryFieldsPanel } from "./EntryFieldsPanel";
import { applyFields, readFields, deleteEntryField } from "./api";
import type { Entry, EntryFields, FieldDefinition } from "./types";
vi.mock("./api", () => ({
  applyFields: vi.fn(),
  readFields: vi.fn(),
  deleteEntryField: vi.fn(),
}));
const entry: Entry = {
  workspaceState: "active",
  id: "entry",
  categoryId: "category",
  typeId: "type",
  authoredName: "Thron",
  displayName: "Thron",
  revision: 1,
  globalRevision: 2,
};
const definition: FieldDefinition = {
  id: "field",
  name: "Eye colour",
  kind: "choice",
  revision: 1,
  retired: false,
  options: [
    { id: "green", label: "Green", retired: true },
    { id: "blue", label: "Blue", retired: false },
  ],
  bindings: [],
};
const snapshot: EntryFields = {
  globalRevision: 2,
  definitions: [definition],
  fields: [{ definition, available: true, value: { kind: "choices", value: ["green"] } }],
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readFields).mockResolvedValue(snapshot);
  vi.mocked(applyFields).mockResolvedValue({ ...snapshot, globalRevision: 3 });
});
function show() {
  const onController = vi.fn();
  render(
    <EntryFieldsPanel
      projectId="project"
      entry={entry}
      disabled={false}
      onController={onController}
      onRevision={vi.fn()}
    />,
  );
  return onController;
}
describe("Field authoring", () => {
  it("keeps a dismissed creation draft and any failed save visible until explicitly cancelled", async () => {
    vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
    const controller = show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Age" } });
    fireEvent(
      screen.getByRole("dialog", { name: "Add field" }),
      new Event("cancel", { cancelable: true }),
    );
    expect(screen.getByRole("button", { name: "Continue Field draft" })).toBeVisible();
    expect(controller).toHaveBeenLastCalledWith(
      expect.objectContaining({ state: "dirty", canSubmit: false }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Continue Field draft" }));
    expect(screen.getByLabelText("new-field-name")).toHaveValue("Age");
    fireEvent.click(screen.getByRole("button", { name: "Create field" }));
    await screen.findByText("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Close Add field" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Continue Field draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel new field" }));
    expect(screen.getByLabelText("new-field-name")).toHaveValue("");
  });
  it("places a custom unit in the quantity control and describes the numeric input accessibly", async () => {
    const numeric = { ...definition, kind: "number" as const, unit: "years", name: "Age" };
    vi.mocked(readFields).mockResolvedValue({
      ...snapshot,
      definitions: [numeric],
      fields: [{ definition: numeric, available: true, value: { kind: "number", value: 48 } }],
    });
    show();
    const value = await screen.findByLabelText("Value: Age");
    expect(value).toHaveValue("48");
    expect(value).toHaveAccessibleDescription("years");
    expect(value.parentElement).toContainElement(screen.getByText("years"));
    const user = userEvent.setup();
    await user.click(value.parentElement!);
    expect(value).toHaveFocus();
    await user.click(screen.getByText("years"));
    expect(value).toHaveFocus();
    expect(screen.queryByText("Unit: years")).not.toBeInTheDocument();
    expect(screen.queryByText("(Number (optional unit))")).not.toBeInTheDocument();
  });
  it("creates a Number and custom unit together and keeps the value numeric", async () => {
    show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    expect(screen.getByLabelText("new-field-kind")).toBeVisible();
    fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Mass" } });
    fireEvent.change(screen.getByLabelText("new-field-kind"), { target: { value: "number" } });
    fireEvent.change(screen.getByLabelText("new-field-unit"), { target: { value: "tons" } });
    fireEvent.change(screen.getByLabelText("new-field-value"), { target: { value: "8000000" } });
    fireEvent.click(screen.getByRole("button", { name: "Create field" }));
    await waitFor(() =>
      expect(applyFields).toHaveBeenCalledWith(
        "project",
        "entry",
        2,
        expect.objectContaining({
          fieldKind: "number",
          unit: "tons",
          value: { kind: "number", value: 8000000 },
        }),
      ),
    );
  });
  it("keeps default field inputs editable and focused while values save", async () => {
    const user = userEvent.setup();
    const a = { ...definition, id: "mass", name: "Mass", kind: "number" as const, unit: "tons" };
    const b = { ...definition, id: "range", name: "Range", kind: "number" as const, unit: "km" };
    const values = {
      globalRevision: 2,
      definitions: [a, b],
      fields: [a, b].map((d) => ({ definition: d, available: true, value: null })),
    };
    vi.mocked(readFields).mockResolvedValue(values);
    let resolve!: (value: EntryFields) => void;
    vi.mocked(applyFields).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    show();
    const mass = await screen.findByLabelText("Value: Mass");
    const range = screen.getByLabelText("Value: Range");
    mass.focus();
    fireEvent.change(mass, { target: { value: "8000000" } });
    await user.tab();
    expect(screen.getByRole("button", { name: "Clear value: Mass" })).toHaveFocus();
    await user.tab();
    await waitFor(() => expect(applyFields).toHaveBeenCalled());
    expect(range).toBeEnabled();
    expect(range).toHaveFocus();
    fireEvent.change(range, { target: { value: "10" } });
    await act(async () =>
      resolve({
        ...values,
        globalRevision: 3,
        fields: [
          { definition: a, available: true, value: { kind: "number", value: 8000000 } },
          values.fields[1],
        ],
      }),
    );
    expect(range).toHaveValue("10");
    expect(range).toHaveFocus();
    vi.mocked(applyFields).mockResolvedValueOnce({
      ...values,
      globalRevision: 4,
      fields: [
        { definition: a, available: true, value: { kind: "number", value: 8000000 } },
        { definition: b, available: true, value: { kind: "number", value: 10 } },
      ],
    });
    await waitFor(() => expect(applyFields).toHaveBeenCalledTimes(2));
    expect(applyFields).toHaveBeenLastCalledWith("project", "entry", 3, {
      kind: "set_values",
      edits: [{ fieldId: "range", value: { kind: "number", value: 10 } }],
    });
    expect(range).toHaveValue("10");
    expect(range).toHaveFocus();
  });
  it("keeps a shared-definition draft when its dialog closes with Escape", async () => {
    const onController = show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
    fireEvent.click(screen.getByRole("button", { name: "Edit definition: Eye colour" }));
    fireEvent.change(screen.getByLabelText("rename-field"), { target: { value: "Eyes" } });
    fireEvent(
      screen.getByRole("dialog", { name: "Edit field definition" }),
      new Event("cancel", { cancelable: true }),
    );
    expect(screen.queryByRole("dialog", { name: "Edit field definition" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close Manage fields" }));
    expect(onController).toHaveBeenLastCalledWith(
      expect.objectContaining({ state: "dirty", canSubmit: false }),
    );
    expect(applyFields).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Continue definition edits" }));
    expect(screen.getByLabelText("rename-field")).toHaveValue("Eyes");
    fireEvent.click(screen.getByRole("button", { name: "Cancel definition edits" }));
    expect(onController).toHaveBeenLastCalledWith(
      expect.objectContaining({ state: "saved", canSubmit: true }),
    );
  });

  it("keeps a failed shared edit visible after closing its dialog", async () => {
    vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
    show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
    fireEvent.click(screen.getByRole("button", { name: "Edit definition: Eye colour" }));
    fireEvent.change(screen.getByLabelText("rename-field"), { target: { value: "Eyes" } });
    fireEvent.click(screen.getByRole("button", { name: "Rename definition" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Close Edit field definition" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Continue definition edits" }));
    expect(screen.getByLabelText("rename-field")).toHaveValue("Eyes");
  });
  it("shows retired authored choices and keeps new fields optional", async () => {
    show();
    await screen.findByLabelText("Value: Eye colour");
    expect(screen.getByLabelText("Value: Eye colour")).toHaveValue("green");
    expect(screen.getByRole("option", { name: "Green (retired)" })).toBeInTheDocument();
    expect(screen.getByLabelText("new-field-name")).not.toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    expect(screen.getByRole("button", { name: "Create field" })).toBeDisabled();
    expect(applyFields).not.toHaveBeenCalled();
  });
  it("creates a local field with its value atomically and clears the form only after success", async () => {
    show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    fireEvent.change(screen.getByLabelText("new-field-name"), {
      target: { value: "Shell diameter" },
    });
    fireEvent.change(screen.getByLabelText("new-field-kind"), { target: { value: "number" } });
    fireEvent.change(screen.getByLabelText("new-field-value"), { target: { value: "900" } });
    fireEvent.click(screen.getByRole("button", { name: "Create field" }));
    await waitFor(() =>
      expect(applyFields).toHaveBeenCalledWith("project", "entry", 2, {
        kind: "create",
        name: "Shell diameter",
        fieldKind: "number",
        provider: { kind: "entry", id: "entry" },
        options: [],
        value: { kind: "number", value: 900 },
      }),
    );
    await waitFor(() => expect(screen.getByLabelText("new-field-name")).toHaveValue(""));
  });
  it("marks unfinished definition drafts as unsaved and preserves them after failure", async () => {
    vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
    const onController = show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Age" } });
    expect(onController).toHaveBeenLastCalledWith(
      expect.objectContaining({ state: "dirty", canSubmit: false }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create field" }));
    await screen.findByText("Disk full");
    expect(screen.getByLabelText("new-field-name")).toHaveValue("Age");
    await waitFor(() =>
      expect(onController).toHaveBeenLastCalledWith(
        expect.objectContaining({ state: "failed", canSubmit: false }),
      ),
    );
  });
  it("promotes the same definition to the current Type without rewriting a value", async () => {
    show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
    fireEvent.click(screen.getByRole("button", { name: "Edit definition: Eye colour" }));
    await act(async () =>
      fireEvent.click(
        screen.getByRole("button", { name: "Make available to this Type", hidden: true }),
      ),
    );
    expect(applyFields).toHaveBeenCalledWith("project", "entry", 2, {
      kind: "bind",
      fieldId: "field",
      provider: { kind: "type", id: "type" },
    });
  });
});

it("suggests reuse with readable provider context and preserves an initial value draft", async () => {
  const local = {
    ...definition,
    kind: "number" as const,
    name: "Age",
    unit: "years",
    options: [],
    bindings: [{ provider: { kind: "entry" as const, id: "entry" }, label: "Thron" }],
  };
  vi.mocked(readFields).mockResolvedValue({ ...snapshot, fields: [], definitions: [local] });
  show();
  await waitFor(() => expect(screen.getByRole("button", { name: "Add field" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Add field" }));
  fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: " age " } });
  expect(screen.getByText(/Age · Number · years · Thron/, { selector: "li" })).toBeVisible();
  fireEvent.change(screen.getByLabelText("new-field-value"), { target: { value: "48" } });
  expect(screen.getByRole("button", { name: "Reuse Age" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("new-field-value"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Reuse Age" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith("project", "entry", 2, {
      kind: "bind",
      fieldId: "field",
      provider: { kind: "entry", id: "entry" },
    }),
  );
});

it("lists current Fields and reusable defaults in separate compact tables", async () => {
  const other = {
    ...definition,
    id: "other",
    name: "Height",
    kind: "number" as const,
    unit: "metres",
  };
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    definitions: [definition, other],
    fields: [
      {
        ...snapshot.fields[0],
        defaultSources: [{ provider: { kind: "type", id: "parent" }, label: "Ancestor type" }],
      },
    ],
  });
  show();
  await screen.findByLabelText("Value: Eye colour");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  const tables = within(screen.getByRole("dialog", { name: "Manage fields" })).getAllByRole(
    "table",
  );
  expect(tables).toHaveLength(2);
  expect(within(tables[0]).getByText("Ancestor type")).toBeVisible();
  expect(within(tables[0]).queryByText("Green")).not.toBeInTheDocument();
  expect(within(tables[1]).getByRole("button", { name: "Add to Entry: Height" })).toBeEnabled();
  expect(screen.queryByLabelText("field-definition")).not.toBeInTheDocument();
  fireEvent.click(within(tables[1]).getByRole("button", { name: "Add to Entry: Height" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith("project", "entry", 2, {
      kind: "bind",
      fieldId: "other",
      provider: { kind: "entry", id: "entry" },
    }),
  );
});

it("hides locally, previews hidden values without writing, and unhides explicitly", async () => {
  const hidden = {
    ...snapshot,
    globalRevision: 3,
    fields: [{ ...snapshot.fields[0], hidden: true }],
  };
  vi.mocked(applyFields)
    .mockResolvedValueOnce(hidden)
    .mockResolvedValueOnce({ ...snapshot, globalRevision: 4 });
  show();
  await screen.findByLabelText("Value: Eye colour");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  fireEvent.click(screen.getByRole("button", { name: "Hide field: Eye colour" }));
  await screen.findByRole("button", { name: "Show field: Eye colour" });
  expect(applyFields).toHaveBeenCalledWith("project", "entry", 2, {
    kind: "set_hidden",
    fieldId: "field",
    hidden: true,
  });
  fireEvent.click(screen.getByRole("button", { name: "Close Manage fields" }));
  expect(screen.queryByLabelText("Value: Eye colour")).not.toBeInTheDocument();
  fireEvent.click(menuItem("Edit", "Field", "Show hidden fields"));
  expect(screen.getByLabelText("Value: Eye colour")).toHaveValue("green");
  expect(applyFields).toHaveBeenCalledTimes(1);
  fireEvent.click(menuItem("Edit", "Field", "Hide hidden fields"));
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  fireEvent.click(screen.getByRole("button", { name: "Show field: Eye colour" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenLastCalledWith("project", "entry", 3, {
      kind: "set_hidden",
      fieldId: "field",
      hidden: false,
    }),
  );
  await screen.findByRole("button", { name: "Hide field: Eye colour" });
});

it("keeps values and exposes failures when hiding fails", async () => {
  vi.mocked(applyFields).mockRejectedValueOnce(new Error("Disk full"));
  show();
  await screen.findByLabelText("Value: Eye colour");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  fireEvent.click(screen.getByRole("button", { name: "Hide field: Eye colour" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
  expect(screen.getByRole("button", { name: "Hide field: Eye colour" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "Close Manage fields" }));
  expect(screen.getByLabelText("Value: Eye colour")).toHaveValue("green");
});

it("reviews local deletion, preserves the value on backup failure, and waits for acknowledgement", async () => {
  vi.mocked(deleteEntryField).mockRejectedValueOnce(new Error("Backup failed"));
  const controller = show();
  await screen.findByLabelText("Value: Eye colour");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  fireEvent.click(screen.getByRole("button", { name: "Delete from Entry: Eye colour" }));
  expect(deleteEntryField).not.toHaveBeenCalled();
  expect(screen.getByText("Current value: Green")).toBeVisible();
  expect(screen.queryByLabelText("Delete Field backup folder")).not.toBeInTheDocument();
  expect(screen.queryByText("Recovery backup folder")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Delete" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Backup failed");
  expect(screen.getByLabelText("Value: Eye colour")).toHaveValue("green");
  let finish!: (value: Awaited<ReturnType<typeof deleteEntryField>>) => void;
  vi.mocked(deleteEntryField).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Delete" }));
  expect(screen.getByRole("button", { name: "Delete" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Close Delete Eye colour from this Entry?" }));
  expect(screen.getByLabelText("Value: Eye colour")).toBeDisabled();
  expect(controller).toHaveBeenLastCalledWith(expect.objectContaining({ state: "saving" }));
  expect(deleteEntryField).toHaveBeenLastCalledWith("project", "entry", "field", 2);
  await act(async () =>
    finish({
      snapshot: { ...snapshot, fields: [], globalRevision: 3 },
      backupPath: "/Backups/recovery",
    }),
  );
  expect(
    screen.queryByRole("dialog", { name: "Delete Eye colour from this Entry?" }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Add to Entry: Eye colour" })).toBeVisible();
  expect(screen.queryByLabelText("Value: Eye colour")).not.toBeInTheDocument();
  expect(controller).toHaveBeenLastCalledWith(expect.objectContaining({ state: "saved" }));
});

it("offers a simple cancel without loading backup settings or deleting", async () => {
  show();
  await screen.findByLabelText("Value: Eye colour");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  fireEvent.click(screen.getByRole("button", { name: "Delete from Entry: Eye colour" }));
  const dialog = screen.getByRole("dialog", { name: "Delete Eye colour from this Entry?" });
  expect(within(dialog).queryByRole("textbox")).not.toBeInTheDocument();
  expect(within(dialog).getByRole("button", { name: "Delete" })).toBeEnabled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(deleteEntryField).not.toHaveBeenCalled();
});

it("never flashes a save button while typing or saving, but offers retry after failure", async () => {
  const numeric = { ...definition, kind: "number" as const, name: "Age" };
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    definitions: [numeric],
    fields: [{ definition: numeric, available: true, value: { kind: "number", value: 48 } }],
  });
  let reject!: (reason: Error) => void;
  vi.mocked(applyFields).mockImplementationOnce(
    () =>
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
  );
  show();
  const value = await screen.findByLabelText("Value: Age");
  fireEvent.change(value, { target: { value: "49" } });
  expect(
    screen.queryByRole("button", { name: /Save field values|Retry saving fields/ }),
  ).not.toBeInTheDocument();
  await waitFor(() => expect(applyFields).toHaveBeenCalled());
  expect(
    screen.queryByRole("button", { name: /Save field values|Retry saving fields/ }),
  ).not.toBeInTheDocument();
  await act(async () => reject(new Error("Disk full")));
  expect(value).toHaveValue("49");
  fireEvent.click(screen.getByRole("button", { name: "Retry saving fields" }));
  await waitFor(() => expect(applyFields).toHaveBeenCalledTimes(2));
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "Retry saving fields" })).not.toBeInTheDocument(),
  );
});
