import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { renderWithMenu as render } from "./desktopMenuTestUtils";
import { ChapterLibrary, type ChapterBrowseState } from "./ChapterLibrary";
import { applyStory, readStory } from "./api";
import { chapterFixture } from "./chapterTestFixtures";
import type { StoryIndex } from "./storyTypes";

vi.mock("./api", () => ({ applyStory: vi.fn(), readStory: vi.fn() }));

let index: StoryIndex;
beforeEach(() => {
  vi.resetAllMocks();
  const chapter = chapterFixture().chapter;
  index = {
    globalRevision: 7,
    chapters: Array.from({ length: 123 }, (_, i) => ({
      ...chapter,
      id: `chapter-${i + 1}`,
      title: `Chapter ${String(i + 1).padStart(3, "0")}`,
      readingRank: i + 1,
      wordCount: i + 1,
      workspaceState: i < 120 ? "active" : i < 122 ? "archived" : "trashed",
    })),
  };
  vi.mocked(readStory).mockImplementation(async () => index);
  vi.mocked(applyStory).mockResolvedValue({ ...chapterFixture(), globalRevision: 8 });
});

function show(locked = false) {
  const onOpen = vi.fn();
  render(
    <ChapterLibrary
      projectId="project"
      onOpen={onOpen}
      onController={vi.fn()}
      onRevision={vi.fn()}
      locked={locked}
    />,
  );
  return { onOpen };
}

it("browses 120 Chapters in bounded pages, jumps to a page and resets pagination for searches", async () => {
  const { onOpen } = show();
  await screen.findByRole("button", { name: "Chapter 001" });
  const results = screen.getByRole("list", { name: "Chapter results" });
  expect(within(results).getAllByRole("listitem")).toHaveLength(20);
  expect(screen.getByText("1–20 of 120")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Next Chapter results" }));
  expect(screen.getByRole("button", { name: "Chapter 021" })).toBeVisible();
  expect(screen.queryByRole("button", { name: "Chapter 001" })).not.toBeInTheDocument();
  expect(screen.getByLabelText("Reading position 21")).toHaveTextContent("21");
  fireEvent.change(screen.getByLabelText("Chapter results page"), { target: { value: "5" } });
  expect(screen.getByText("101–120 of 120")).toBeVisible();
  fireEvent.change(screen.getByRole("searchbox", { name: "Find a Chapter" }), {
    target: { value: "Chapter 005" },
  });
  expect(within(results).getAllByRole("listitem")).toHaveLength(1);
  expect(screen.getByLabelText("Reading position 5")).toHaveTextContent("05");
  fireEvent.click(screen.getByRole("button", { name: "Chapter 005" }));
  expect(onOpen).toHaveBeenCalledWith("chapter-5");
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "#119" } });
  expect(screen.getByRole("button", { name: "Chapter 119" })).toBeVisible();
  expect(applyStory).not.toHaveBeenCalled();
});

it("sorts the display without changing canonical reading positions or saved order", async () => {
  index.chapters[0].title = "Zulu";
  index.chapters[119].title = "Alpha";
  show();
  await screen.findByRole("button", { name: "Zulu" });
  fireEvent.change(screen.getByLabelText("Display order"), { target: { value: "title" } });
  const first = within(screen.getByRole("list", { name: "Chapter results" })).getAllByRole(
    "listitem",
  )[0];
  expect(within(first).getByRole("button", { name: "Alpha" })).toBeVisible();
  expect(within(first).getByLabelText("Reading position 120")).toHaveTextContent("120");
  fireEvent.change(screen.getByLabelText("Display order"), { target: { value: "words" } });
  expect(screen.getByText(/keep your reading order unchanged/)).toBeVisible();
  expect(applyStory).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("Display order"), { target: { value: "reading" } });
  expect(
    within(screen.getByRole("list", { name: "Chapter results" })).getAllByRole("listitem")[0],
  ).toHaveTextContent("Zulu");
});

it("moves a searched Chapter to the full reading-order position rather than a filtered neighbor", async () => {
  show();
  await screen.findByRole("button", { name: "Chapter 001" });
  fireEvent.change(screen.getByLabelText("Display order"), { target: { value: "words" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Chapter 099" } });
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter 099 in reading order" }));
  const dialog = screen.getByRole("dialog", { name: "Move Chapter" });
  expect(within(dialog).getByLabelText("New reading position")).toHaveValue(99);
  expect(within(dialog).getByRole("button", { name: "Move Chapter" })).toBeDisabled();
  fireEvent.change(within(dialog).getByLabelText("New reading position"), {
    target: { value: "0" },
  });
  expect(within(dialog).getByRole("button", { name: "Move Chapter" })).toBeDisabled();
  fireEvent.change(within(dialog).getByLabelText("New reading position"), {
    target: { value: "2" },
  });
  expect(within(dialog).getByText("Place before Chapter 002.")).toBeVisible();
  fireEvent.click(within(dialog).getByRole("button", { name: "Move Chapter" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 7, {
      kind: "move",
      chapterId: "chapter-99",
      beforeId: "chapter-2",
    }),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
});

it("moves forward across page boundaries using the Chapter after the requested position", async () => {
  show();
  await screen.findByRole("button", { name: "Chapter 001" });
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter 001 in reading order" }));
  fireEvent.change(screen.getByLabelText("New reading position"), { target: { value: "100" } });
  expect(screen.getByText("Place before Chapter 101.")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 7, {
      kind: "move",
      chapterId: "chapter-1",
      beforeId: "chapter-101",
    }),
  );
});

it("moves to the end of Writing while excluding archived and trashed Chapters as move targets", async () => {
  show();
  await screen.findByRole("button", { name: "Chapter 001" });
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter 001 in reading order" }));
  fireEvent.change(screen.getByLabelText("New reading position"), { target: { value: "120" } });
  expect(screen.getByText("Place at the end of Writing.")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter" }));
  await waitFor(() =>
    expect(applyStory).toHaveBeenCalledWith("project", 7, {
      kind: "move",
      chapterId: "chapter-1",
      beforeId: null,
    }),
  );
});

it("keeps failed moves and the chosen position reviewable", async () => {
  vi.mocked(applyStory).mockRejectedValue(new Error("Disk full"));
  const first = show();
  expect(first.onOpen).not.toHaveBeenCalled();
  await screen.findByRole("button", { name: "Chapter 001" });
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter 001 in reading order" }));
  fireEvent.change(screen.getByLabelText("New reading position"), { target: { value: "2" } });
  fireEvent.click(screen.getByRole("button", { name: "Move Chapter" }));
  const dialog = screen.getByRole("dialog", { name: "Move Chapter" });
  expect(await within(dialog).findByRole("alert")).toHaveTextContent("Disk full");
  expect(within(dialog).getByLabelText("New reading position")).toHaveValue(2);
});

it("disables create, reorder and restore while locked", async () => {
  show(true);
  await screen.findByRole("button", { name: "Chapter 001" });
  expect(screen.getByRole("button", { name: "New Chapter" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Move Chapter 001 in reading order" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Trash" }));
  expect(screen.getByRole("button", { name: "Restore" })).toBeDisabled();
  expect(applyStory).not.toHaveBeenCalled();
});

it("recovers from no matches by clearing the query", async () => {
  show();
  await screen.findByRole("button", { name: "Chapter 001" });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "missing" } });
  expect(screen.getByText(/No Chapters match/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
  expect(screen.getByRole("button", { name: "Chapter 001" })).toBeVisible();
  expect(applyStory).not.toHaveBeenCalled();
});

it.each([
  { query: "Chapter", sort: "reading" },
  { query: "#1", sort: "words" },
] as const)(
  "returns focus to search after moving the trigger out of the $sort results without resetting the view",
  async ({ query, sort }) => {
    vi.mocked(applyStory).mockImplementation(async () => {
      const moved = index.chapters[0];
      index = {
        ...index,
        globalRevision: 8,
        chapters: [...index.chapters.slice(1, 120), moved, ...index.chapters.slice(120)].map(
          (chapter, position) => ({ ...chapter, readingRank: position + 1 }),
        ),
      };
      return { ...chapterFixture(), chapter: { ...moved, readingRank: 120 }, globalRevision: 8 };
    });
    show();
    await screen.findByRole("button", { name: "Chapter 001" });
    const search = screen.getByRole("searchbox", { name: "Find a Chapter" });
    fireEvent.change(search, { target: { value: query } });
    fireEvent.change(screen.getByLabelText("Display order"), { target: { value: sort } });
    const trigger = screen.getByRole("button", { name: "Move Chapter 001 in reading order" });
    trigger.focus();
    fireEvent.click(trigger);
    const position = screen.getByLabelText("New reading position");
    position.focus();
    fireEvent.change(position, { target: { value: "120" } });
    fireEvent.click(screen.getByRole("button", { name: "Move Chapter" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "Chapter 001" })).not.toBeInTheDocument();
    await waitFor(() => expect(search).toHaveFocus());
    expect(search).toHaveValue(query);
    expect(screen.getByLabelText("Display order")).toHaveValue(sort);
    expect(screen.getByRole("button", { name: "Writing" })).toHaveAttribute("aria-pressed", "true");
  },
);

it("reports and restores the library view, query, display order and page after a remount", async () => {
  index.chapters = index.chapters.map((chapter) => ({ ...chapter, workspaceState: "archived" }));
  const onBrowseStateChange = vi.fn();
  const common = {
    projectId: "project",
    onOpen: vi.fn(),
    onController: vi.fn(),
    onRevision: vi.fn(),
    onBrowseStateChange,
  };
  const first = render(<ChapterLibrary {...common} />);
  await waitFor(() => expect(screen.getByRole("button", { name: "New Chapter" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Archive" }));
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Chapter" } });
  fireEvent.change(screen.getByLabelText("Display order"), { target: { value: "words" } });
  fireEvent.click(screen.getByRole("button", { name: "Next Chapter results" }));
  const held: ChapterBrowseState = { view: "archived", query: "Chapter", sort: "words", page: 1 };
  expect(onBrowseStateChange).toHaveBeenLastCalledWith(held);
  first.unmount();
  const returned = render(<ChapterLibrary {...common} initialBrowseState={held} />);
  expect(await screen.findByRole("button", { name: "Chapter 103" })).toBeVisible();
  expect(screen.getByRole("searchbox")).toHaveValue("Chapter");
  expect(screen.getByLabelText("Display order")).toHaveValue("words");
  expect(screen.getByRole("button", { name: "Archive" })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByText("21–40 of 123")).toBeVisible();
  returned.rerender(<ChapterLibrary {...common} initialBrowseState={held} />);
  expect(screen.getByText("21–40 of 123")).toBeVisible();
  expect(applyStory).not.toHaveBeenCalled();
});
