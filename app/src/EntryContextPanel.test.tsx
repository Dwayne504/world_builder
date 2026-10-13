import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { EntryContextContent, EntryContextPanel } from "./EntryContextPanel";
import {
  getEntry,
  listTypes,
  readEntryDescription,
  readFields,
  readRelationships,
  readSpatial,
} from "./api";
import { descriptionFixture } from "./entryDescriptionTestFixtures";
import { deferred } from "./chapterTestFixtures";
import type { Entry, EntryField } from "./types";

vi.mock("./api", () => ({
  getEntry: vi.fn(),
  listTypes: vi.fn(),
  readEntryDescription: vi.fn(),
  readFields: vi.fn(),
  readRelationships: vi.fn(),
  readSpatial: vi.fn(),
}));
const entry: Entry = {
  id: "one",
  categoryId: "category",
  typeId: null,
  displayName: "Floating library",
  authoredName: "Floating library",
  workspaceState: "active",
  revision: 1,
  globalRevision: 1,
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getEntry).mockResolvedValue(entry);
  vi.mocked(listTypes).mockResolvedValue([]);
  vi.mocked(readEntryDescription).mockResolvedValue(
    descriptionFixture("A library that sails between worlds."),
  );
  vi.mocked(readFields).mockResolvedValue({ globalRevision: 1, fields: [], definitions: [] });
  vi.mocked(readRelationships).mockResolvedValue({
    globalRevision: 1,
    relationships: [],
    definitions: [],
    entries: [],
  });
  vi.mocked(readSpatial).mockResolvedValue({ globalRevision: 1, entries: [], defaults: [] });
});
function field(id: string, value: EntryField["value"]): EntryField {
  return {
    definition: {
      id,
      name: id,
      kind: "short_text",
      retired: false,
      revision: 1,
      options: [],
      bindings: [],
    },
    available: true,
    value,
  };
}

it("reads descriptions and typed facts without object text, hidden facts or lost zero values", async () => {
  const reference = field("Port", {
    kind: "entry_references",
    value: { expectedIds: ["ref"], items: [{ kind: "existing", referenceId: "ref" }] },
  });
  reference.referenceTargets = [
    { id: "ref", targetId: "two", label: "Old harbour", workspaceState: "archived" },
  ];
  const mass = field("Mass", { kind: "number", value: 0 });
  mass.definition.unit = "tons";
  vi.mocked(readFields).mockResolvedValue({
    globalRevision: 1,
    definitions: [],
    fields: [
      mass,
      field("Sunk", { kind: "boolean", value: false }),
      reference,
      field("Lore", {
        kind: "rich_text",
        value: { ...descriptionFixture("Saved lore").document!, schemaVersion: 1 },
      }),
      { ...field("Secret", { kind: "text", value: "Hidden words" }), hidden: true },
    ],
  });
  const follow = vi.fn();
  render(<EntryContextContent projectId="project" entryId="one" onPreview={follow} />);
  expect(await screen.findByText("A library that sails between worlds.")).toBeVisible();
  fireEvent.click(screen.getByText("Facts"));
  expect(screen.getByText("0 tons")).toBeVisible();
  expect(screen.getByText("No")).toBeVisible();
  expect(screen.getByText("Saved lore")).toBeVisible();
  expect(screen.queryByText("Hidden words")).not.toBeInTheDocument();
  expect(document.body).not.toHaveTextContent("[object Object]");
  fireEvent.click(screen.getByRole("button", { name: "Old harbour" }));
  expect(follow).toHaveBeenCalledWith("two");
});

it("ignores a late response for the previous reference and supports retry without leaving writing", async () => {
  const old = deferred<Entry>();
  vi.mocked(getEntry)
    .mockReturnValueOnce(old.promise)
    .mockRejectedValueOnce(new Error("Busy"))
    .mockResolvedValue(entry);
  const view = render(<EntryContextContent projectId="project" entryId="old" />);
  view.rerender(<EntryContextContent projectId="project" entryId="one" />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Your writing is still here");
  await act(async () => old.resolve({ ...entry, id: "old" }));
  expect(screen.queryByText("A library that sails between worlds.")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Retry reference" }));
  expect(await screen.findByText("A library that sails between worlds.")).toBeVisible();
});

it("provides keyboard dismissal back to the opener and an explicit guarded editing action", async () => {
  const opener = document.createElement("button");
  opener.textContent = "Linked Entry";
  document.body.append(opener);
  const close = vi.fn(),
    open = vi.fn();
  const props = {
    projectId: "project",
    entryId: "one",
    categories: [],
    opener,
    onClose: close,
    onPreview: vi.fn(),
    onOpen: open,
    openDisabled: true,
  };
  const view = render(<EntryContextPanel {...props} />);
  const panel = screen.getByRole("complementary", { name: "Entry reference" });
  expect(within(panel).getByRole("button", { name: "Close reference" })).toHaveFocus();
  await screen.findByRole("heading", { name: "Floating library" });
  expect(within(panel).getByRole("button", { name: "Open Entry for editing" })).toBeDisabled();
  view.rerender(<EntryContextPanel {...props} openDisabled={false} />);
  fireEvent.click(within(panel).getByRole("button", { name: "Open Entry for editing" }));
  expect(open).toHaveBeenCalledWith("one");
  const handledEscape = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
  handledEscape.preventDefault();
  fireEvent(window, handledEscape);
  expect(close).not.toHaveBeenCalled();
  const dialog = document.createElement("dialog");
  dialog.setAttribute("open", "");
  document.body.append(dialog);
  fireEvent.keyDown(window, { key: "Escape" });
  expect(close).not.toHaveBeenCalled();
  dialog.remove();
  fireEvent.keyDown(window, { key: "Escape" });
  await waitFor(() => expect(opener).toHaveFocus());
  expect(close).toHaveBeenCalledTimes(1);
  view.unmount();
  opener.remove();
});
