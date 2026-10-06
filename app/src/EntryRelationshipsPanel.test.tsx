import { menuItem, renderWithMenu as render } from "./desktopMenuTestUtils";
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { EntryRelationshipsPanel } from "./EntryRelationshipsPanel";
import { applyRelationships, readRelationships } from "./api";
import type { FieldsController } from "./EntryFieldsPanel";
import type { EntryRelationships, Relationship } from "./types";

vi.mock("./api", () => ({ readRelationships: vi.fn(), applyRelationships: vi.fn() }));
const owner = { id: "thron", label: "Thron", workspaceState: "active" };
const blade = { id: "blade", label: "Singularity Blade", workspaceState: "active" };
const relationship: Relationship = {
  id: "r1",
  definitionId: "ownership",
  source: owner,
  target: blade,
  note: "A gift",
  ended: false,
  workspaceState: "active",
  revision: 1,
  warnings: [],
};
function snapshot(): EntryRelationships {
  return {
    globalRevision: 3,
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
    relationships: [{ ...relationship }],
    entries: [
      { ...owner, categoryName: "People" },
      { ...blade, categoryName: "Objects" },
    ],
  };
}
let controller: FieldsController;
let revision: number;
const navigate = vi.fn();
async function mount(entryId = "thron") {
  render(
    <EntryRelationshipsPanel
      projectId="project"
      entryId={entryId}
      categories={[]}
      disabled={false}
      onController={(next) => {
        controller = next;
      }}
      onRevision={(next) => {
        revision = Math.max(revision, next);
      }}
      getRevision={() => revision}
      onNavigate={navigate}
    />,
  );
  fireEvent.click(
    await screen.findByRole("button", {
      name: entryId === "thron" ? "Show owns relationships" : "Show is owned by relationships",
    }),
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  revision = 3;
  vi.mocked(readRelationships).mockResolvedValue(snapshot());
  vi.mocked(applyRelationships).mockResolvedValue({ ...snapshot(), globalRevision: 4 });
});
function change(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}
function openActions() {
  fireEvent.click(screen.getByLabelText("Note and actions · has note"));
}

it("deduplicates only presented current relationships and offers an explicit full view", async () => {
  const past = { ...relationship, id: "past", ended: true };
  vi.mocked(readRelationships).mockResolvedValue({
    ...snapshot(),
    relationships: [relationship, past],
  });
  const props = {
    projectId: "project",
    entryId: "thron",
    categories: [],
    disabled: false,
    onController: vi.fn(),
    onRevision: vi.fn(),
    getRevision: () => 3,
    onNavigate: navigate,
  };
  const { rerender } = render(
    <EntryRelationshipsPanel {...props} presentedRelationships={["r1"]} />,
  );
  const toggle = await screen.findByLabelText("Show all relationships, including those in Fields");
  expect(
    screen.getAllByRole("button", { name: "Show owns relationships", hidden: true }),
  ).toHaveLength(1);
  fireEvent.click(toggle);
  expect(
    screen.getAllByRole("button", { name: "Show owns relationships", hidden: true }),
  ).toHaveLength(2);
  fireEvent.click(toggle);
  rerender(<EntryRelationshipsPanel {...props} presentedRelationships={[]} />);
  expect(
    screen.queryByLabelText("Show all relationships, including those in Fields"),
  ).not.toBeInTheDocument();
  expect(
    screen.getAllByRole("button", { name: "Show owns relationships", hidden: true }),
  ).toHaveLength(2);
});

it("refreshes after a companion commit without rebasing an unfinished note", async () => {
  const committed = vi.fn();
  const props = {
    projectId: "project",
    entryId: "thron",
    categories: [],
    disabled: false,
    onController: vi.fn(),
    onRevision: vi.fn(),
    getRevision: () => 3,
    onNavigate: navigate,
    onCommitted: committed,
  };
  const { rerender } = render(<EntryRelationshipsPanel {...props} refreshKey={0} />);
  fireEvent.click(await screen.findByRole("button", { name: "Show owns relationships" }));
  openActions();
  vi.mocked(applyRelationships).mockRejectedValueOnce(new Error("Write failed"));
  change("Note: owns Singularity Blade", "Keep this draft");
  fireEvent.click(screen.getByRole("button", { name: "Save note" }));
  await screen.findAllByText(/Write failed/);
  const reads = vi.mocked(readRelationships).mock.calls.length;
  rerender(<EntryRelationshipsPanel {...props} refreshKey={1} />);
  expect(readRelationships).toHaveBeenCalledTimes(reads);
  expect(screen.getByLabelText("Note: owns Singularity Blade")).toHaveValue("Keep this draft");
  expect(committed).not.toHaveBeenCalled();
});

it("shows the inverse from the other Entry and navigates by stable ID", async () => {
  await mount("blade");
  expect(screen.getByText("is owned by")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Thron" }).closest("li")).not.toHaveTextContent(
    "Ownership",
  );
  fireEvent.click(screen.getByRole("button", { name: "Thron" }));
  expect(navigate).toHaveBeenCalledWith("thron");
  expect(applyRelationships).not.toHaveBeenCalled();
});
it("keeps conflicts visible and only replaces relationships explicitly selected", async () => {
  const data = snapshot();
  data.relationships[0].warnings = ["Expected one owner; found two."];
  vi.mocked(readRelationships).mockResolvedValue(data);
  await mount();
  expect(screen.getByRole("status")).toHaveTextContent("found two");
  fireEvent.click(screen.getByRole("button", { name: "Add relationship" }));
  change("Relationship definition", "ownership");
  change("Other Entry", "blade");
  fireEvent.click(screen.getByRole("button", { name: "Create relationship" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenLastCalledWith(
      "project",
      "thron",
      3,
      expect.objectContaining({ kind: "connect", replace: [] }),
    ),
  );
  await waitFor(() => expect(controller.state).toBe("saved"));
  fireEvent.click(screen.getByRole("button", { name: "Add relationship" }));
  change("Relationship definition", "ownership");
  change("Other Entry", "blade");
  fireEvent.click(screen.getByText("Replace an existing relationship…"));
  fireEvent.click(screen.getByLabelText("End owns Singularity Blade"));
  fireEvent.click(screen.getByRole("button", { name: "End selected and connect" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenLastCalledWith(
      "project",
      "thron",
      4,
      expect.objectContaining({ replace: ["r1"] }),
    ),
  );
});
it("quick-creates an optional stub and links it with one command without navigating away", async () => {
  await mount();
  fireEvent.click(screen.getByRole("button", { name: "Add relationship" }));
  change("Relationship definition", "ownership");
  change("Connect to", "new");
  fireEvent.click(screen.getByRole("button", { name: "Create relationship" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenCalledWith(
      "project",
      "thron",
      3,
      expect.objectContaining({ other: { kind: "create", name: null, categoryId: null } }),
    ),
  );
  expect(navigate).not.toHaveBeenCalled();
});
it("keeps failed creation and hidden dialog drafts until explicit cancellation", async () => {
  vi.mocked(applyRelationships).mockRejectedValue(new Error("Disk full"));
  await mount();
  fireEvent.click(screen.getByRole("button", { name: "Add relationship" }));
  change("Relationship definition", "ownership");
  change("Other Entry", "blade");
  change("Note (optional)", "Do not lose this");
  fireEvent.click(screen.getByRole("button", { name: "Create relationship" }));
  await waitFor(() => expect(controller.state).toBe("failed"));
  expect(screen.getByLabelText("Note (optional)")).toHaveValue("Do not lose this");
  fireEvent.click(screen.getByRole("button", { name: "Close Add relationship" }));
  expect(screen.getByRole("button", { name: "Continue relationship draft" })).toBeInTheDocument();
  expect(controller.canSubmit).toBe(false);
  await act(async () => {
    expect(await controller.submit()).toEqual({ kind: "failed" });
  });
  fireEvent.click(screen.getByRole("button", { name: "Continue relationship draft" }));
  fireEvent.click(screen.getByRole("button", { name: "Cancel relationship" }));
  expect(controller.canSubmit).toBe(true);
});
it("does not report a note saved before acknowledgement and waits when closing", async () => {
  let resolve!: (data: EntryRelationships) => void;
  vi.mocked(applyRelationships).mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  await mount();
  openActions();
  change("Note: owns Singularity Blade", "A note in flight");
  fireEvent.click(screen.getByRole("button", { name: "Save note" }));
  await waitFor(() => expect(controller.state).toBe("saving"));
  expect(screen.getByLabelText("Note: owns Singularity Blade")).toHaveValue("A note in flight");
  let closed = false;
  let outcome: Promise<unknown>;
  act(() => {
    outcome = controller.submit().then(() => {
      closed = true;
    });
  });
  expect(closed).toBe(false);
  const updated = snapshot();
  updated.globalRevision = 4;
  updated.relationships[0].note = "A note in flight";
  await act(async () => {
    resolve(updated);
    await outcome;
  });
  expect(closed).toBe(true);
  expect(controller.state).toBe("saved");
  expect(applyRelationships).toHaveBeenCalledTimes(1);
});
it("preserves failed notes through explicit reload and retries against the new revision", async () => {
  vi.mocked(applyRelationships).mockRejectedValueOnce(new Error("Stale revision"));
  await mount();
  openActions();
  change("Note: owns Singularity Blade", "Local note");
  fireEvent.click(screen.getByRole("button", { name: "Save note" }));
  await waitFor(() => expect(controller.state).toBe("failed"));
  expect(screen.getByLabelText("Note: owns Singularity Blade")).toHaveValue("Local note");
  vi.mocked(readRelationships).mockResolvedValue({ ...snapshot(), globalRevision: 9 });
  fireEvent.click(screen.getByRole("button", { name: "Reload relationships" }));
  await waitFor(() => expect(controller.state).toBe("dirty"));
  fireEvent.click(screen.getByRole("button", { name: "Save note" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenLastCalledWith("project", "thron", 9, {
      kind: "set_note",
      id: "r1",
      note: "Local note",
    }),
  );
});
it("retains a newer note draft when an older note is acknowledged", async () => {
  let resolve!: (data: EntryRelationships) => void;
  vi.mocked(applyRelationships).mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  await mount();
  openActions();
  change("Note: owns Singularity Blade", "First draft");
  fireEvent.click(screen.getByRole("button", { name: "Save note" }));
  await waitFor(() => expect(controller.state).toBe("saving"));
  const input = screen.getByLabelText("Note: owns Singularity Blade");
  input.focus();
  expect(input).toBeEnabled();
  change("Note: owns Singularity Blade", "Newer draft");
  const updated = snapshot();
  updated.relationships[0].note = "First draft";
  await act(async () => {
    resolve(updated);
  });
  expect(screen.getByLabelText("Note: owns Singularity Blade")).toHaveValue("Newer draft");
  expect(controller.state).toBe("dirty");
  expect(input).toHaveFocus();
});
it("creates symmetric definitions with matching labels and expectations", async () => {
  await mount();
  fireEvent.click(menuItem("Edit", "Relationship", "Manage relationships…"));
  change("Definition name", "Alliance");
  change("Direction", "symmetric");
  change("Relationship label", "allied with");
  const dialog = screen.getByRole("dialog", { name: "Relationship definitions" });
  fireEvent.click(within(dialog).getByText("Optional expectations"));
  change("Expected maximum per Entry", "2");
  fireEvent.click(screen.getByRole("button", { name: "Create definition" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenCalledWith("project", "thron", 3, {
      kind: "create_definition",
      draft: {
        name: "Alliance",
        directed: false,
        forwardLabel: "allied with",
        inverseLabel: "allied with",
        expectedTargetsPerSource: 2,
        expectedSourcesPerTarget: 2,
      },
    }),
  );
});

it("finds a relationship target beyond one hundred Entries and retains it through filtering and dismissal", async () => {
  const next = snapshot();
  next.entries = Array.from({ length: 125 }, (_, index) => ({
    id: `person-${index}`,
    label: `Person ${index}`,
    categoryName: "People",
  }));
  vi.mocked(readRelationships).mockResolvedValue(next);
  await mount();
  fireEvent.click(screen.getByRole("button", { name: "Add relationship" }));
  change("Relationship definition", "ownership");
  const target = screen.getByRole("combobox", { name: "Other Entry" });
  expect(within(target).getAllByRole("option")).toHaveLength(21);
  change("Search Other Entry", "Person 124");
  change("Other Entry", "person-124");
  change("Search Other Entry", "Missing person");
  expect(target).toHaveValue("person-124");
  fireEvent.click(screen.getByRole("button", { name: "Close Add relationship" }));
  expect(controller.canSubmit).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Continue relationship draft" }));
  expect(target).toHaveValue("person-124");
  fireEvent.click(screen.getByRole("button", { name: "Create relationship" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenCalledWith(
      "project",
      "thron",
      3,
      expect.objectContaining({
        kind: "connect",
        other: { kind: "existing", id: "person-124" },
      }),
    ),
  );
});
it("ends and restores the same relationship identity", async () => {
  const ended = snapshot();
  ended.globalRevision = 4;
  ended.relationships[0].ended = true;
  vi.mocked(applyRelationships).mockResolvedValueOnce(ended);
  await mount();
  openActions();
  fireEvent.click(screen.getByRole("button", { name: "End relationship" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenCalledWith("project", "thron", 3, {
      kind: "set_ended",
      id: "r1",
      ended: true,
    }),
  );
  await screen.findByText("Past and inactive relationships (1)");
  fireEvent.click(screen.getByText("Past and inactive relationships (1)"));
  fireEvent.click(screen.getByRole("button", { name: "Show owns relationships" }));
  openActions();
  fireEvent.click(screen.getByRole("button", { name: "Restore relationship" }));
  await waitFor(() =>
    expect(applyRelationships).toHaveBeenLastCalledWith("project", "thron", 4, {
      kind: "set_ended",
      id: "r1",
      ended: false,
    }),
  );
});

it("groups targets, searches within a group, and preserves separate notes while collapsing", async () => {
  const data = snapshot();
  data.relationships.push({
    ...relationship,
    id: "r2",
    target: { id: "second", label: "Second object", workspaceState: "active" },
    note: "Separate note",
  });
  vi.mocked(readRelationships).mockResolvedValue(data);
  await mount();
  expect(screen.getAllByRole("button", { name: "Hide owns relationships" })).toHaveLength(1);
  fireEvent.change(screen.getByLabelText("Find an Entry in owns"), { target: { value: "second" } });
  expect(screen.queryByRole("button", { name: "Singularity Blade" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByLabelText("Note and actions · has note"));
  expect(screen.getByLabelText("Note: owns Second object")).toHaveValue("Separate note");
  fireEvent.click(screen.getByRole("button", { name: "Hide owns relationships" }));
  expect(screen.getByLabelText("Note: owns Second object")).not.toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Show owns relationships" }));
  expect(screen.getByLabelText("Note: owns Second object")).toBeVisible();
  fireEvent.change(screen.getByLabelText("Find an Entry in owns"), { target: { value: "nobody" } });
  expect(screen.getByText("No Entries match this search.")).toBeVisible();
  expect(applyRelationships).not.toHaveBeenCalled();
});
