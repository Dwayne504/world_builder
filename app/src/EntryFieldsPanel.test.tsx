import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EntryFieldsPanel } from "./EntryFieldsPanel";
import { applyFields, readFields } from "./api";
import type { Entry, EntryFields, FieldDefinition } from "./types";
vi.mock("./api", () => ({ applyFields: vi.fn(), readFields: vi.fn() }));
const entry: Entry = {
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
  it("keeps a shared-definition draft when its dialog closes with Escape", async () => {
    const onController = show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByRole("button", { name: "Manage fields" }));
    fireEvent.change(screen.getByLabelText("field-definition"), { target: { value: "field" } });
    fireEvent.change(screen.getByLabelText("rename-field"), { target: { value: "Eyes" } });
    fireEvent(
      screen.getByRole("dialog", { name: "Manage fields" }),
      new Event("cancel", { cancelable: true }),
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
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
    fireEvent.click(screen.getByRole("button", { name: "Manage fields" }));
    fireEvent.change(screen.getByLabelText("field-definition"), { target: { value: "field" } });
    fireEvent.change(screen.getByLabelText("rename-field"), { target: { value: "Eyes" } });
    fireEvent.click(screen.getByRole("button", { name: "Rename definition" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Close Manage fields" }));
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
    fireEvent.click(screen.getByText("Add a field", { selector: "summary" }));
    expect(screen.getByRole("button", { name: "Add field" })).toBeDisabled();
    expect(applyFields).not.toHaveBeenCalled();
  });
  it("creates a local field with its value atomically and clears the form only after success", async () => {
    show();
    await screen.findByLabelText("Value: Eye colour");
    fireEvent.click(screen.getByText("Add a field", { selector: "summary" }));
    fireEvent.change(screen.getByLabelText("new-field-name"), {
      target: { value: "Shell diameter" },
    });
    fireEvent.change(screen.getByLabelText("new-field-kind"), { target: { value: "number" } });
    fireEvent.change(screen.getByLabelText("new-field-value"), { target: { value: "900" } });
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
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
    fireEvent.click(screen.getByText("Add a field", { selector: "summary" }));
    fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Age" } });
    expect(onController).toHaveBeenLastCalledWith(
      expect.objectContaining({ state: "dirty", canSubmit: false }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
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
    fireEvent.click(screen.getByRole("button", { name: "Manage fields" }));
    fireEvent.change(screen.getByLabelText("field-definition"), { target: { value: "field" } });
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
