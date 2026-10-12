import { useState } from "react";
import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { EntryReferenceValue } from "./EntryReferenceValue";
import { applyFields, listEntries, readFields } from "./api";
import { useEntryFields, valueDraft } from "./useEntryFields";
import type { Entry, EntryField, EntryFields, ReferenceSelection } from "./types";
vi.mock("./api", () => ({
  listEntries: vi.fn(),
  readFields: vi.fn(),
  applyFields: vi.fn(),
  deleteEntryField: vi.fn(),
}));
const field: EntryField = {
  definition: {
    id: "field",
    name: "See also",
    kind: "entry_reference",
    revision: 1,
    retired: false,
    options: [],
    bindings: [],
  },
  available: true,
  value: null,
  referenceTargets: [],
};
const snapshot: EntryFields = {
  globalRevision: 1,
  definitions: [field.definition],
  fields: [field],
};
function entry(index: number): Entry {
  return {
    id: `entry-${index}`,
    displayName: `Reference ${index}`,
    authoredName: null,
    categoryId: "category",
    typeId: null,
    revision: 1,
    globalRevision: 1,
    workspaceState: "active",
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(listEntries).mockResolvedValue(Array.from({ length: 1005 }, (_, i) => entry(i)));
  vi.mocked(readFields).mockResolvedValue(snapshot);
});
function show(current = field, disabled = false, onNavigate = vi.fn()) {
  const changed = vi.fn();
  function Wrapper() {
    const [draft, setDraft] = useState(String(valueDraft(current.value)));
    return (
      <EntryReferenceValue
        projectId="project"
        field={current}
        draft={draft}
        disabled={disabled}
        onNavigate={onNavigate}
        onChange={(next) => {
          changed(next);
          setDraft(next);
        }}
      />
    );
  }
  render(<Wrapper />);
  return changed;
}
async function expand() {
  await act(async () => fireEvent.click(screen.getByText("Edit references")));
  await waitFor(() => expect(listEntries).toHaveBeenCalled());
}
it("bounds a thousand choices, searches beyond the first page and preserves selection order", async () => {
  const changed = show();
  await expand();
  const choices = await screen.findByRole("list", { name: "Available Entries: See also" });
  await waitFor(() => expect(within(choices).getAllByRole("listitem")).toHaveLength(10));
  fireEvent.change(screen.getByLabelText("Find Entry for See also"), {
    target: { value: "Reference 1004" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add reference: Reference 1004" }));
  expect(JSON.parse(changed.mock.lastCall![0])).toEqual({
    expectedIds: [],
    items: [{ kind: "entry", entryId: "entry-1004" }],
  });
  fireEvent.change(screen.getByLabelText("Find Entry for See also"), {
    target: { value: "Reference 20" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add reference: Reference 20" }));
  fireEvent.click(screen.getByRole("button", { name: "Move up: Reference 20" }));
  expect(
    JSON.parse(changed.mock.lastCall![0]).items.map((item: { entryId: string }) => item.entryId),
  ).toEqual(["entry-20", "entry-1004"]);
  fireEvent.click(screen.getByRole("button", { name: "Remove reference: Reference 20" }));
  expect(JSON.parse(changed.mock.lastCall![0]).items).toHaveLength(1);
});
it("keeps inactive and unresolved references intelligible and navigates only resolved IDs", async () => {
  const navigate = vi.fn();
  const selected: EntryField = {
    ...field,
    value: {
      kind: "entry_references",
      value: {
        expectedIds: ["old", "gone"],
        items: [
          { kind: "existing", referenceId: "old" },
          { kind: "existing", referenceId: "gone" },
        ],
      },
    },
    referenceTargets: [
      { id: "old", targetId: "entry-1", label: "Renamed Entry", workspaceState: "trashed" },
      { id: "gone", targetId: null, label: "Old snapshot", workspaceState: null },
    ],
  };
  const changed = show(selected, false, navigate);
  fireEvent.click(screen.getByRole("button", { name: "Renamed Entry" }));
  expect(navigate).toHaveBeenCalledWith("entry-1");
  expect(screen.getByText(/trashed/)).toBeVisible();
  expect(screen.getByText("Old snapshot")).toBeVisible();
  expect(screen.queryByRole("button", { name: "Old snapshot" })).not.toBeInTheDocument();
  await expand();
  fireEvent.click(screen.getByRole("button", { name: "Move up: Old snapshot" }));
  expect(JSON.parse(changed.mock.lastCall![0])).toEqual({
    expectedIds: ["old", "gone"],
    items: [
      { kind: "existing", referenceId: "gone" },
      { kind: "existing", referenceId: "old" },
    ],
  });
  fireEvent.click(screen.getByRole("button", { name: "Use saved references" }));
  expect(JSON.parse(changed.mock.lastCall![0])).toEqual(selected.value?.value);
});
it("shows load failures and retry without changing the saved selection", async () => {
  vi.mocked(listEntries).mockRejectedValueOnce(new Error("Unavailable list"));
  const changed = show();
  await expand();
  expect(await screen.findByRole("alert")).toHaveTextContent("Unavailable list");
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Retry Entry list" })));
  expect(await screen.findByRole("button", { name: "Add reference: Reference 0" })).toBeEnabled();
  expect(changed).not.toHaveBeenCalled();
});
it("keeps search usable but selection changes disabled while a save is pending", async () => {
  const changed = show(field, true);
  await expand();
  expect(await screen.findByRole("button", { name: "Add reference: Reference 0" })).toBeDisabled();
  const search = screen.getByLabelText("Find Entry for See also");
  expect(search).toBeEnabled();
  fireEvent.change(search, { target: { value: "Reference 1004" } });
  expect(screen.getByRole("button", { name: "Add reference: Reference 1004" })).toBeDisabled();
  expect(changed).not.toHaveBeenCalled();
});
it("bounds selected references and finds a retained target beyond the first page", () => {
  const targets = Array.from({ length: 1001 }, (_, i) => ({
    id: `ref-${i}`,
    targetId: `entry-${i}`,
    label: `Saved ${i}`,
    workspaceState: "archived" as const,
  }));
  show({
    ...field,
    referenceTargets: targets,
    value: {
      kind: "entry_references",
      value: {
        expectedIds: targets.map((t) => t.id),
        items: targets.map((t) => ({ kind: "existing", referenceId: t.id })),
      },
    },
  });
  const selected = screen.getByRole("list", { name: "Selected Entries: See also" });
  expect(within(selected).getAllByRole("listitem")).toHaveLength(10);
  fireEvent.change(screen.getByLabelText("Find selected Entries"), {
    target: { value: "Saved 1000" },
  });
  expect(within(selected).getAllByRole("listitem")).toHaveLength(1);
  expect(screen.getByRole("button", { name: "Saved 1000" })).toBeVisible();
});
it("keeps an old reference expectation across a failed save and explicit reload", async () => {
  const initial: ReferenceSelection = {
    expectedIds: ["old"],
    items: [{ kind: "existing", referenceId: "old" }],
  };
  const saved: EntryField = { ...field, value: { kind: "entry_references", value: initial } };
  vi.mocked(readFields).mockResolvedValue({ ...snapshot, fields: [saved] });
  vi.mocked(applyFields).mockRejectedValue(new Error("References changed"));
  const { result } = renderHook(() => useEntryFields("project", "owner", 1, vi.fn(), () => 20));
  await waitFor(() => expect(result.current.snapshot).not.toBeNull());
  const draft = JSON.stringify({ ...initial, items: [] });
  act(() => result.current.change("field", draft));
  await act(async () => {
    await result.current.submit();
  });
  expect(result.current.state).toBe("failed");
  vi.mocked(readFields).mockResolvedValue({
    ...snapshot,
    globalRevision: 21,
    fields: [
      {
        ...saved,
        value: {
          kind: "entry_references",
          value: { expectedIds: ["new"], items: [{ kind: "existing", referenceId: "new" }] },
        },
      },
    ],
  });
  await act(async () => {
    await result.current.reload();
    await result.current.submit();
  });
  expect(result.current.drafts.field).toBe(draft);
  expect(applyFields).toHaveBeenLastCalledWith("project", "owner", 21, {
    kind: "set_values",
    edits: [
      {
        fieldId: "field",
        value: { kind: "entry_references", value: { expectedIds: ["old"], items: [] } },
      },
    ],
  });
});
