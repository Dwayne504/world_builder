import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { ChapterEditor } from "./ChapterEditor";
import { ChapterLibrary } from "./ChapterLibrary";
import { applyStory, listEntries, readStory, readChapter } from "./api";
import { chapterFixture, textDocument } from "./chapterTestFixtures";
import { storeChapterDraft } from "./chapterRecovery";
import type { ChapterController } from "./storyTypes";
vi.mock("./api", () => ({
  applyStory: vi.fn(),
  listEntries: vi.fn(),
  readStory: vi.fn(),
  readChapter: vi.fn(),
  readFields: vi.fn(),
  getEntry: vi.fn(),
  readRelationships: vi.fn(),
  readSpatial: vi.fn(),
}));
let controller: ChapterController;
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  vi.mocked(listEntries).mockResolvedValue([]);
});
function show(initial = chapterFixture()) {
  return render(
    <ChapterEditor
      projectId="project"
      initial={initial}
      onController={(value) => {
        controller = value;
      }}
      onChanged={vi.fn()}
      onBack={vi.fn()}
      onEntry={vi.fn()}
      positions={{}}
    />,
  );
}
it("opens manuscript first, keeps three documents separate, and hides context without remounting prose", async () => {
  show();
  const prose = await screen.findByRole("textbox", { name: "Manuscript" });
  expect(prose).toHaveTextContent("Thron arrived.");
  fireEvent.click(screen.getByRole("button", { name: "Hide context" }));
  expect(screen.getByRole("textbox", { name: "Manuscript" })).toBe(prose);
  expect(screen.queryByRole("complementary", { name: "Chapter context" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Plan" }));
  expect(screen.getByRole("textbox", { name: "Chapter plan" })).not.toHaveTextContent("Thron");
  fireEvent.click(screen.getByRole("tab", { name: "Manuscript" }));
  expect(screen.getByRole("textbox", { name: "Manuscript" })).toBe(prose);
  expect(screen.getByText("2 words · manuscript")).toBeVisible();
});
it("preserves unsupported original documents while leaving Notes editable", async () => {
  const initial = chapterFixture();
  initial.documents[0] = {
    ...initial.documents[0],
    content: null,
    readOnlyReason: "Newer document version",
    originalJson: "untouched newer document",
  };
  show(initial);
  expect(screen.getByRole("alert")).toHaveTextContent("Newer document version");
  expect(screen.queryByRole("textbox", { name: "Manuscript" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByText("Original document"));
  expect(screen.getByLabelText("manuscript original document")).toHaveValue(
    "untouched newer document",
  );
  fireEvent.click(screen.getByRole("tab", { name: "Notes" }));
  expect(await screen.findByRole("textbox", { name: "Chapter notes" })).toBeVisible();
  expect(applyStory).not.toHaveBeenCalled();
});
it("reviews a recovery draft before replacing the saved manuscript", async () => {
  const initial = chapterFixture();
  storeChapterDraft("project", "chapter", 2, {
    documents: { manuscript: textDocument("Unsaved words") },
  });
  vi.mocked(applyStory).mockResolvedValue({ ...initial, globalRevision: 4 });
  show(initial);
  expect(screen.queryByRole("textbox", { name: "Manuscript" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Review recovered draft" }));
  const dialog = screen.getByRole("dialog", { name: "Review recovered writing" });
  expect(within(dialog).getByLabelText("Saved writing")).toHaveValue("Thron arrived.\n");
  expect(within(dialog).getByLabelText("Recovered writing")).toHaveValue("Unsaved words\n");
  expect(applyStory).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Use recovered draft" }));
  expect(await screen.findByRole("textbox", { name: "Manuscript" })).toHaveTextContent(
    "Unsaved words",
  );
  await act(async () => {
    await controller.submit();
  });
  expect(applyStory).toHaveBeenCalledWith("project", 3, expect.objectContaining({ kind: "save" }));
});
it("shows a failed-save warning, keeps the title draft, and explicitly retries", async () => {
  const initial = chapterFixture();
  vi.mocked(applyStory)
    .mockRejectedValueOnce(new Error("Disk full"))
    .mockResolvedValue({ ...initial, globalRevision: 4 });
  show(initial);
  fireEvent.change(screen.getByLabelText("Chapter title"), { target: { value: "My chapter" } });
  await act(async () => {
    await controller.submit();
  });
  expect(screen.getByRole("alert")).toHaveTextContent("Changes are not being saved.");
  expect(screen.getByLabelText("Chapter title")).toHaveValue("My chapter");
  fireEvent.click(screen.getByRole("button", { name: "Retry saving" }));
  await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
});

it("keeps damaged local recovery data read-only and requires explicit review", async () => {
  localStorage.setItem(
    "worldcrafter:chapter-recovery:project:chapter",
    "do not drop this damaged original",
  );
  show();
  fireEvent.click(screen.getByRole("button", { name: "Review recovered draft" }));
  expect(screen.getByLabelText("Original recovery data")).toHaveValue(
    "do not drop this damaged original",
  );
  expect(screen.getByRole("button", { name: "Use recovered draft" })).toBeDisabled();
  expect(controller.state).toBe("dirty");
  expect(controller.canSubmit).toBe(false);
  expect(applyStory).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Keep saved Chapter" }));
  expect(await screen.findByRole("textbox", { name: "Manuscript" })).toHaveTextContent(
    "Thron arrived.",
  );
});

it("compares a failed draft with the latest saved Chapter before explicit restoration", async () => {
  vi.mocked(applyStory).mockRejectedValue(new Error("Stale revision"));
  vi.mocked(readChapter).mockResolvedValue({ ...chapterFixture(), globalRevision: 7 });
  show();
  fireEvent.change(screen.getByLabelText("Chapter title"), {
    target: { value: "My unsaved title" },
  });
  await act(async () => {
    await controller.submit();
  });
  fireEvent.click(screen.getByRole("button", { name: "Review saved version" }));
  const review = await screen.findByRole("dialog", { name: "Review recovered writing" });
  expect(within(review).getByText("My unsaved title")).toBeInTheDocument();
  expect(applyStory).toHaveBeenCalledTimes(1);
  expect(controller.canSubmit).toBe(false);
  vi.mocked(applyStory).mockResolvedValue({ ...chapterFixture(), globalRevision: 8 });
  fireEvent.click(within(review).getByRole("button", { name: "Use recovered draft" }));
  await act(async () => {
    await controller.submit();
  });
  expect(applyStory).toHaveBeenLastCalledWith(
    "project",
    7,
    expect.objectContaining({ title: "My unsaved title" }),
  );
});
it("creates an optional Chapter without a Book, reorders by stable identity, and restores Trash", async () => {
  const initial = chapterFixture();
  vi.mocked(readStory).mockResolvedValue({
    globalRevision: 3,
    chapters: [
      initial.chapter,
      { ...initial.chapter, id: "next", title: "Next", readingRank: 2 },
      {
        ...initial.chapter,
        id: "trash",
        title: "Put aside",
        workspaceState: "trashed",
        readingRank: 3,
      },
    ],
  });
  vi.mocked(applyStory).mockResolvedValue(initial);
  const onOpen = vi.fn();
  render(
    <ChapterLibrary
      projectId="project"
      onOpen={onOpen}
      onController={vi.fn()}
      onRevision={vi.fn()}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Move Next up" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 3, {
      kind: "move",
      chapterId: "next",
      beforeId: "chapter",
    }),
  );
  await waitFor(() => expect(screen.getByRole("button", { name: "Trash" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Trash" }));
  fireEvent.click(screen.getByRole("button", { name: "Restore" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 3, {
      kind: "set_state",
      chapterId: "trash",
      state: "active",
    }),
  );
  await waitFor(() => expect(screen.getByRole("button", { name: "New Chapter" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "New Chapter" }));
  await waitFor(() => expect(onOpen).toHaveBeenCalledWith("chapter", expect.anything()));
});
it("searches a bounded list and creates a role-free canonical Story link", async () => {
  const initial = chapterFixture();
  vi.mocked(listEntries).mockResolvedValue(
    Array.from({ length: 30 }, (_, i) => ({
      id: `e${i}`,
      displayName: `Person ${i}`,
      authoredName: `Person ${i}`,
      categoryId: "people",
      typeId: null,
      revision: 1,
      globalRevision: 3,
    })),
  );
  // Keep the response empty to isolate the command from the asynchronous Entry preview.
  vi.mocked(applyStory).mockResolvedValue({ ...initial, globalRevision: 4 });
  show(initial);
  fireEvent.click(screen.getByRole("button", { name: "Link Entry" }));
  const dialog = screen.getByRole("dialog", { name: "Link world material" });
  await within(dialog).findByRole("button", { name: "Person 0" });
  expect(within(dialog).getAllByRole("button", { name: /^Person/ })).toHaveLength(10);
  fireEvent.change(within(dialog).getByLabelText("Find an Entry"), {
    target: { value: "Person 29" },
  });
  fireEvent.click(within(dialog).getByRole("button", { name: "Person 29" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 3, {
      kind: "set_link",
      chapterId: "chapter",
      entryId: "e29",
      roleIds: [],
    }),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
});
