import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { EditorView } from "@tiptap/pm/view";
import { EntryDescriptionPanel } from "./EntryDescriptionPanel";
import { readEntryDescription, saveEntryDescription } from "./api";
import { descriptionFixture } from "./entryDescriptionTestFixtures";
import { deferred, textDocument } from "./chapterTestFixtures";
import { storeEntryDescriptionDraft } from "./entryDescriptionRecovery";
import type { FieldsController } from "./EntryFieldsPanel";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";

vi.mock("./api", () => ({ readEntryDescription: vi.fn(), saveEntryDescription: vi.fn() }));
let controller: FieldsController;
afterEach(() => vi.restoreAllMocks());
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  vi.mocked(readEntryDescription).mockResolvedValue(descriptionFixture());
});
function show() {
  return render(
    <EntryDescriptionPanel
      projectId="project"
      entryId="entry"
      onRevision={vi.fn()}
      getRevision={() => 3}
      onController={(next) => {
        controller = next;
      }}
    />,
  );
}
async function write(prose: HTMLElement, text: string) {
  await act(async () => {
    prose.querySelector("p")!.textContent = text;
    fireEvent.input(prose, { inputType: "insertText", data: text });
  });
}

it("keeps a description optional, and opening the writing area does not create content", async () => {
  show();
  const add = await screen.findByRole("button", { name: "Add description" });
  expect(screen.queryByRole("textbox", { name: "Entry description" })).not.toBeInTheDocument();
  fireEvent.click(add);
  expect(await screen.findByRole("textbox", { name: "Entry description" })).toBeVisible();
  await act(async () => {
    await controller.submit();
  });
  expect(saveEntryDescription).not.toHaveBeenCalled();
});

it("keeps focus, editor identity and undo through save acknowledgements", async () => {
  // jsdom has no Range geometry; keep this test about focus and editor history.
  vi.spyOn(EditorView.prototype, "coordsAtPos").mockReturnValue({
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
  });
  vi.mocked(readEntryDescription).mockResolvedValue(descriptionFixture("A sailor"));
  const pending = deferred<EntryDescriptionSnapshot>();
  vi.mocked(saveEntryDescription)
    .mockReturnValueOnce(pending.promise)
    .mockImplementation(
      async (_project, _entry, expected, _documentRevision, _version, content) => ({
        ...descriptionFixture("A sailor with a story"),
        globalRevision: expected + 1,
        document: { ...descriptionFixture("A sailor with a story").document!, content },
      }),
    );
  show();
  const prose = await screen.findByRole("textbox", { name: "Entry description" });
  act(() => prose.focus());
  await write(prose, "A sailor with a story");
  await waitFor(() => expect(saveEntryDescription).toHaveBeenCalledTimes(1));
  await act(async () =>
    pending.resolve({ ...descriptionFixture("A sailor with a story"), globalRevision: 4 }),
  );
  expect(screen.getByRole("textbox", { name: "Entry description" })).toBe(prose);
  expect(prose).toHaveFocus();
  expect(prose).toHaveTextContent("A sailor with a story");
  fireEvent.click(screen.getByRole("button", { name: "Undo writing" }));
  expect(prose).toHaveTextContent("A sailor");
  expect(prose).not.toHaveTextContent("with a story");
});

it("offers explicit comparison of failed writing and blocks leaving until it is resolved", async () => {
  vi.mocked(readEntryDescription).mockResolvedValue(descriptionFixture("Saved lore"));
  vi.mocked(saveEntryDescription).mockRejectedValue(new Error("Stale revision"));
  show();
  const prose = await screen.findByRole("textbox", { name: "Entry description" });
  await write(prose, "Keep my writing");
  await act(async () => {
    await controller.submit();
  });
  expect(screen.getByRole("alert")).toHaveTextContent("Description changes are not being saved");
  expect(prose).toHaveTextContent("Keep my writing");
  vi.mocked(readEntryDescription).mockResolvedValue({
    ...descriptionFixture("Latest saved lore"),
    globalRevision: 7,
  });
  fireEvent.click(screen.getByRole("button", { name: "Review saved description" }));
  fireEvent.click(await screen.findByRole("button", { name: "Review description draft" }));
  const dialog = screen.getByRole("dialog", { name: "Review description draft" });
  expect(within(dialog).getByLabelText("Saved description")).toHaveValue("Latest saved lore\n");
  expect(within(dialog).getByLabelText("Recovered description")).toHaveValue("Keep my writing\n");
  expect(controller.canSubmit).toBe(false);
  fireEvent.click(within(dialog).getByRole("button", { name: "Keep saved description" }));
  expect(await screen.findByRole("textbox", { name: "Entry description" })).toHaveTextContent(
    "Latest saved lore",
  );
  expect(controller.state).toBe("saved");
});

it("reviews a local draft before restoring it into an otherwise empty Entry", async () => {
  storeEntryDescriptionDraft("project", "entry", 2, textDocument("Recovered biography"));
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Review description draft" }));
  expect(screen.queryByRole("textbox", { name: "Entry description" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Use description draft" }));
  expect(await screen.findByRole("textbox", { name: "Entry description" })).toHaveTextContent(
    "Recovered biography",
  );
  expect(controller.state).toBe("dirty");
});

it("shows unsupported saved documents with their untouched original and no editable surface", async () => {
  const initial = descriptionFixture("Preserved readable text");
  initial.document = {
    ...initial.document!,
    content: null,
    schemaVersion: 99,
    readOnlyReason: "Newer description version",
    originalJson: '{"schema":"future"}',
  };
  vi.mocked(readEntryDescription).mockResolvedValue(initial);
  show();
  expect(await screen.findByRole("alert")).toHaveTextContent("Newer description version");
  fireEvent.click(screen.getByText("Original description"));
  expect(screen.getByLabelText("Original description data")).toHaveValue('{"schema":"future"}');
  expect(screen.queryByRole("textbox", { name: "Entry description" })).not.toBeInTheDocument();
  expect(saveEntryDescription).not.toHaveBeenCalled();
});

it("preserves damaged emergency drafts until explicitly reviewed and discarded", async () => {
  localStorage.setItem(
    "worldcrafter:entry-description-recovery:project:entry",
    "original damaged data",
  );
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Review description draft" }));
  expect(screen.getByLabelText("Original description recovery data")).toHaveValue(
    "original damaged data",
  );
  expect(screen.getByRole("button", { name: "Use description draft" })).toBeDisabled();
  expect(controller.canSubmit).toBe(false);
  expect(localStorage.getItem("worldcrafter:entry-description-recovery:project:entry")).toBe(
    "original damaged data",
  );
  fireEvent.click(screen.getByRole("button", { name: "Keep saved description" }));
  expect(await screen.findByRole("button", { name: "Add description" })).toBeVisible();
  expect(saveEntryDescription).not.toHaveBeenCalled();
});

it("presents inactive descriptions read-only", async () => {
  vi.mocked(readEntryDescription).mockResolvedValue({
    ...descriptionFixture("Archived lore"),
    workspaceState: "archived",
  });
  show();
  expect(await screen.findByRole("textbox", { name: "Entry description" })).toHaveAttribute(
    "contenteditable",
    "false",
  );
  expect(screen.getByRole("button", { name: "Bold" })).toBeDisabled();
});
