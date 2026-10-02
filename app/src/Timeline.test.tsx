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
import { applyTimeline, listEntries, readStory, readTimeline } from "./api";
import { Timeline, TimelineUsage } from "./Timeline";
import { useTimeline } from "./useTimeline";
import {
  dateLabel,
  initialTimelineView,
  occurrenceDraft,
  type TimelineSnapshot,
  type Occurrence,
  type TimelineView,
} from "./timelineTypes";
import type { ChapterController } from "./storyTypes";
import { deferred } from "./chapterTestFixtures";
vi.mock("./api", () => ({
  applyTimeline: vi.fn(),
  readTimeline: vi.fn(),
  listEntries: vi.fn(),
  readStory: vi.fn(),
}));
const occurrence = (id = "moment"): Occurrence => ({
  id,
  title: "Arrival",
  notes: "Before the story begins",
  date: { year: 0, month: 1, day: 40 },
  eventEntry: null,
  entries: [],
  chapters: [],
  workspaceState: "active",
});
const fixture = (): TimelineSnapshot => ({
  globalRevision: 3,
  calendar: {
    name: "Orbit",
    eraLabel: "AL",
    months: [
      { name: "Dawn", days: 40 },
      { name: "Dusk", days: 8 },
    ],
  },
  occurrences: [occurrence()],
});
let controller: ChapterController;
const onEntry = vi.fn(),
  onChapter = vi.fn();
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readTimeline).mockResolvedValue(fixture());
  vi.mocked(listEntries).mockResolvedValue([]);
  vi.mocked(readStory).mockResolvedValue({ globalRevision: 3, chapters: [] });
  vi.mocked(applyTimeline).mockImplementation(async (_project, revision, command) => {
    const s = fixture();
    s.globalRevision = revision + 1;
    if (command.kind === "create")
      s.occurrences.push({ ...occurrence("new"), title: "", date: null, notes: "" });
    if (command.kind === "save")
      s.occurrences = s.occurrences.map((o) =>
        o.id === command.id
          ? {
              ...o,
              title: command.draft.title,
              notes: command.draft.notes,
              date: command.draft.date,
            }
          : o,
      );
    if (command.kind === "configure_calendar") s.calendar = command.calendar;
    if (command.kind === "set_state") s.occurrences[0].workspaceState = command.state;
    return s;
  });
});
function Host({ initial = initialTimelineView }: { initial?: TimelineView }) {
  const [view, setView] = useState(initial);
  return (
    <Timeline
      projectId="project"
      view={view}
      onViewChange={setView}
      onController={(value) => {
        controller = value;
      }}
      onRevision={vi.fn()}
      onEntry={onEntry}
      onChapter={onChapter}
      locked={false}
    />
  );
}
it("shows a compact fictional date, permits unnamed undated stubs, and opens the acknowledged identity", async () => {
  render(<Host />);
  expect(await screen.findByText("40 Dawn, 0 AL")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Add occurrence" }));
  expect(await screen.findByRole("dialog", { name: "[Untitled occurrence]" })).toBeVisible();
  expect(screen.getByLabelText("Occurrence title (optional)")).toHaveValue("");
  expect(screen.getByRole("checkbox", { name: "Give this occurrence a date" })).not.toBeChecked();
  expect(applyTimeline).toHaveBeenCalledWith("project", 3, { kind: "create" });
});
it("autosaves without remounting or losing focus and drains newer notes before close", async () => {
  const pending = deferred<TimelineSnapshot>();
  vi.mocked(applyTimeline).mockReturnValueOnce(pending.promise);
  render(<Host initial={{ ...initialTimelineView, occurrenceId: "moment" }} />);
  const input = await screen.findByRole("textbox", { name: "Notes" });
  input.focus();
  fireEvent.change(input, { target: { value: "First sentence" } });
  await waitFor(() => {
    expect(applyTimeline).toHaveBeenCalledTimes(1);
    expect(controller.state).toBe("saving");
  });
  fireEvent.change(input, { target: { value: "First sentence. More words." } });
  let flush!: Promise<unknown>;
  act(() => {
    flush = controller.submit();
  });
  await act(async () => {
    pending.resolve({ ...fixture(), globalRevision: 4 });
    await flush;
  });
  expect(applyTimeline).toHaveBeenLastCalledWith(
    "project",
    4,
    expect.objectContaining({
      kind: "save",
      draft: expect.objectContaining({ notes: "First sentence. More words." }),
    }),
  );
  expect(screen.getByRole("textbox", { name: "Notes" })).toBe(input);
  expect(input).toHaveFocus();
  expect(controller.state).toBe("saved");
});
it("retains invalid dates and failed edits and offers explicit retry instead of closing", async () => {
  render(<Host initial={{ ...initialTimelineView, occurrenceId: "moment" }} />);
  const day = await screen.findByLabelText("Day");
  fireEvent.change(day, { target: { value: "41" } });
  fireEvent.click(screen.getByRole("button", { name: "Close Arrival" }));
  await waitFor(() => expect(controller.canSubmit).toBe(false));
  expect(applyTimeline).not.toHaveBeenCalled();
  expect(day).toHaveValue(41);
  expect(screen.getByRole("dialog")).toBeVisible();
  vi.mocked(applyTimeline).mockRejectedValueOnce(new Error("Disk full"));
  fireEvent.change(day, { target: { value: "39" } });
  await act(async () => {
    await controller.submit();
  });
  expect(screen.getByText("Disk full")).toBeVisible();
  expect(day).toHaveValue(39);
  fireEvent.click(screen.getByRole("button", { name: "Retry save" }));
  await waitFor(() => expect(controller.state).toBe("saved"));
});
it("makes calendar rules explicit and prevents changing lengths of already used months", async () => {
  render(<Host />);
  fireEvent.click(await screen.findByRole("button", { name: "Calendar settings" }));
  const dialog = screen.getByRole("dialog", { name: "World calendar" });
  expect(within(dialog).getAllByLabelText("Days")[0]).toBeDisabled();
  expect(within(dialog).getByRole("button", { name: "Add month" })).toBeDisabled();
  fireEvent.change(within(dialog).getByLabelText("Month 1 name"), {
    target: { value: "First light" },
  });
  expect(applyTimeline).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Save calendar" }));
  await waitFor(() => expect(dialog).not.toBeVisible());
  expect(applyTimeline).toHaveBeenCalledWith(
    "project",
    3,
    expect.objectContaining({
      kind: "configure_calendar",
      calendar: expect.objectContaining({
        months: [
          { name: "First light", days: 40 },
          { name: "Dusk", days: 8 },
        ],
      }),
    }),
  );
});
it("allows undated writing before calendar setup and cancels a calendar draft safely", async () => {
  vi.mocked(readTimeline).mockResolvedValue({
    ...fixture(),
    calendar: null,
    occurrences: [{ ...occurrence(), date: null }],
  });
  render(<Host />);
  fireEvent.click(await screen.findByRole("button", { name: "Set up calendar" }));
  fireEvent.change(screen.getByLabelText("Calendar name"), { target: { value: "Draft calendar" } });
  fireEvent.click(screen.getByRole("button", { name: "Cancel calendar edits" }));
  await waitFor(() => expect(controller.state).toBe("saved"));
  expect(applyTimeline).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: /Arrival.*Moment/ }));
  expect(
    await screen.findByRole("checkbox", { name: "Give this occurrence a date" }),
  ).toBeDisabled();
  expect(screen.getByRole("textbox", { name: "Notes" })).toBeEnabled();
});
it("bounds lists, searches occurrence notes, and filters only explicit links", async () => {
  const s = fixture();
  s.occurrences = Array.from({ length: 25 }, (_, i) => ({
    ...occurrence(String(i)),
    title: `Moment ${i}`,
    notes: i === 24 ? "Needle in notes" : "",
    entries: i === 24 ? [{ id: "traveller", label: "Traveller", workspaceState: "active" }] : [],
  }));
  vi.mocked(readTimeline).mockResolvedValue(s);
  render(<Host />);
  await screen.findByText("25 occurrences · Orbit");
  expect(screen.getAllByRole("button", { name: /Moment \d+.*Moment/ })).toHaveLength(10);
  fireEvent.click(screen.getByRole("button", { name: "Show 10 more" }));
  expect(screen.getAllByRole("button", { name: /Moment \d+.*Moment/ })).toHaveLength(20);
  fireEvent.change(screen.getByLabelText("Search timeline"), { target: { value: "Needle" } });
  expect(screen.getByText("1 occurrences · Orbit")).toBeVisible();
  expect(screen.getByRole("button", { name: /Moment 24/ })).toBeVisible();
});
it("keeps inactive occurrences read-only and restores without erasing notes", async () => {
  vi.mocked(readTimeline).mockResolvedValue({
    ...fixture(),
    occurrences: [{ ...occurrence(), workspaceState: "trashed" }],
  });
  render(<Host initial={{ ...initialTimelineView, occurrenceId: "moment", state: "trashed" }} />);
  expect(await screen.findByRole("textbox", { name: "Notes" })).toBeDisabled();
  fireEvent.click(screen.getByText("Archive & Trash"));
  fireEvent.click(screen.getByRole("button", { name: "Restore occurrence" }));
  await waitFor(() =>
    expect(applyTimeline).toHaveBeenCalledWith("project", 3, {
      kind: "set_state",
      id: "moment",
      state: "active",
    }),
  );
});
it("shows Entry and Chapter backlinks without inferring links from notes", async () => {
  const s = fixture();
  s.occurrences[0].notes = "Traveller mentioned in prose";
  s.occurrences.push({
    ...occurrence("linked"),
    title: "Linked event",
    eventEntry: { id: "traveller", label: "Traveller", workspaceState: "active" },
    chapters: [{ id: "chapter", label: "Flashback", workspaceState: "active" }],
  });
  vi.mocked(readTimeline).mockResolvedValue(s);
  const onOpen = vi.fn();
  const { rerender } = render(
    <TimelineUsage projectId="project" entryId="traveller" onOpen={onOpen} />,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Linked event" }));
  expect(onOpen).toHaveBeenCalledWith("linked");
  expect(screen.queryByRole("button", { name: "Arrival" })).not.toBeInTheDocument();
  rerender(<TimelineUsage projectId="project" chapterId="chapter" onOpen={onOpen} />);
  expect(await screen.findByRole("button", { name: "Linked event" })).toBeVisible();
});
it("does not auto-retry stale edits or replace them when the author keeps typing", async () => {
  vi.mocked(applyTimeline).mockRejectedValue(new Error("Stale revision"));
  const { result } = renderHook(() => useTimeline("project", vi.fn()));
  await waitFor(() => expect(result.current.snapshot).not.toBeNull());
  act(() => {
    result.current.edit("moment");
    result.current.change({ ...occurrenceDraft(occurrence()), notes: "Keep me" });
  });
  await act(async () => {
    await result.current.flush();
  });
  act(() => result.current.change({ ...occurrenceDraft(occurrence()), notes: "Keep me too" }));
  expect(result.current.state).toBe("failed");
  expect(result.current.draft?.notes).toBe("Keep me too");
  expect(applyTimeline).toHaveBeenCalledTimes(1);
  expect(dateLabel({ year: -3, month: 2, day: 8 }, fixture().calendar)).toBe("8 Dusk, -3 AL");
});
