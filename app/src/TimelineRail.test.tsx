import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { TimelineRail } from "./TimelineRail";
import { groupTimelineOccurrences } from "./TimelineRail.helpers";
import type { Occurrence, WorldCalendar } from "./timelineTypes";

const calendar: WorldCalendar = {
  name: "Orbit",
  eraLabel: "AL",
  months: [
    { name: "Dawn", days: 40 },
    { name: "Dusk", days: 8 },
  ],
};
const moment = (id: string, year: number | null = -3): Occurrence => ({
  id,
  title: `Moment ${id}`,
  notes: "",
  date: year === null ? null : { year, month: 1, day: 1 },
  eventEntry: null,
  entries: [],
  chapters: [],
  workspaceState: "active",
});

let callbacks: Map<number, FrameRequestCallback>;
let mediaChanged: (() => void) | undefined;
let motion: {
  matches: boolean;
  addEventListener: ReturnType<typeof vi.fn>;
  removeEventListener: ReturnType<typeof vi.fn>;
};
beforeEach(() => {
  callbacks = new Map();
  let nextId = 0;
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn((callback: FrameRequestCallback) => {
      callbacks.set(++nextId, callback);
      return nextId;
    }),
  );
  vi.stubGlobal(
    "cancelAnimationFrame",
    vi.fn((id: number) => callbacks.delete(id)),
  );
  motion = {
    matches: false,
    addEventListener: vi.fn((_event, listener) => {
      mediaChanged = listener;
    }),
    removeEventListener: vi.fn(),
  };
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => motion),
  );
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function flushFrames() {
  act(() => {
    const pending = [...callbacks.values()];
    callbacks.clear();
    pending.forEach((callback) => callback(0));
  });
}

function rail(
  items = [moment("first"), moment("second"), moment("zero", 0), moment("idea", null)],
) {
  const onSelect = vi.fn();
  const rendered = render(
    <TimelineRail
      occurrences={items}
      calendar={calendar}
      selectedId="second"
      viewKey="initial"
      disabled={false}
      onSelect={onSelect}
    />,
  );
  return { ...rendered, onSelect };
}

it("keeps fictional coordinate groups and original ordering, including negative years and undated ideas", () => {
  const earliest = moment("ancient", -1_000_000);
  const first = moment("first");
  const sameDay = moment("second");
  const zero = moment("zero", 0);
  const laterMonth = { ...moment("other-month", 0), date: { year: 0, month: 2, day: 1 } };
  const last = moment("future", 1_000_000);
  const undated = moment("idea", null);
  const items = [earliest, first, sameDay, zero, laterMonth, last, undated];
  const groups = groupTimelineOccurrences(items);
  expect(groups.dated.map((group) => group.key)).toEqual([
    "-1000000/1/1",
    "-3/1/1",
    "0/1/1",
    "0/2/1",
    "1000000/1/1",
  ]);
  expect(groups.dated[1].occurrences).toEqual([first, sameDay]);
  expect(groups.undated).toEqual([undated]);
  expect(items[0]).toBe(earliest);
});

it("labels same-day clusters without inventing times and keeps undated selection separate", async () => {
  const { onSelect } = rail();
  const chronology = screen.getByRole("region", { name: "Chronological timeline" });
  expect(within(chronology).getAllByRole("heading", { name: "1 Dawn, -3 AL" })).toHaveLength(1);
  expect(within(chronology).getByText("2 moments · same day")).toBeVisible();
  expect(screen.getByText("Event spacing, not to scale")).toBeVisible();
  expect(within(chronology).queryByRole("button", { name: /Moment idea/ })).not.toBeInTheDocument();
  expect(within(chronology).getByRole("button", { name: /Moment second/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  const idea = within(screen.getByRole("region", { name: "Undated occurrences" })).getByRole(
    "button",
    { name: "Moment idea · Moment · Undated" },
  );
  await userEvent.click(idea);
  expect(onSelect).toHaveBeenCalledWith("idea");
});

it("supports keyboard navigation and activation without requiring pointer proximity", async () => {
  const { onSelect } = rail();
  const first = screen.getByRole("button", { name: /Moment first/ });
  const second = screen.getByRole("button", { name: /Moment second/ });
  first.focus();
  fireEvent.keyDown(first, { key: "ArrowRight" });
  expect(second).toHaveFocus();
  fireEvent.keyDown(second, { key: "End" });
  expect(screen.getByRole("button", { name: /Moment zero/ })).toHaveFocus();
  fireEvent.keyDown(screen.getByRole("button", { name: /Moment zero/ }), { key: "Home" });
  expect(first).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  expect(onSelect).toHaveBeenCalledWith("first");
});

it("responds to a nearby pointer before hover using fixed anchors and clears emphasis on leave", () => {
  rail([moment("first")]);
  const chronology = screen.getByRole("region", { name: "Chronological timeline" });
  const button = within(chronology).getByRole("button");
  vi.spyOn(button.querySelector(".timeline-rail-anchor")!, "getBoundingClientRect").mockReturnValue(
    { left: 90, top: 90, width: 20, height: 20 } as DOMRect,
  );
  fireEvent(
    chronology,
    new MouseEvent("pointermove", { bubbles: true, clientX: 170, clientY: 100 }),
  );
  flushFrames();
  expect(button).toHaveAttribute("data-near", "true");
  expect(Number(button.style.getPropertyValue("--timeline-nearness"))).toBeGreaterThan(0.5);
  expect(button.style.width).toBe("");
  expect(button.style.height).toBe("");
  fireEvent.pointerLeave(chronology);
  expect(button.style.getPropertyValue("--timeline-nearness")).toBe("");
  expect(button).not.toHaveAttribute("data-near");
});

it("disables proximity work for reduced motion, including preference changes, and uses instant rail scrolling", () => {
  motion.matches = true;
  const { unmount } = rail([moment("first")]);
  const chronology = screen.getByRole("region", { name: "Chronological timeline" });
  fireEvent(
    chronology,
    new MouseEvent("pointermove", { bubbles: true, clientX: 100, clientY: 100 }),
  );
  expect(requestAnimationFrame).not.toHaveBeenCalled();
  const scrollBy = vi.fn();
  Object.defineProperty(chronology, "scrollBy", { value: scrollBy });
  fireEvent.click(screen.getByRole("button", { name: "Scroll to later moments" }));
  expect(scrollBy).toHaveBeenCalledWith(expect.objectContaining({ behavior: "instant" }));
  act(() => {
    motion.matches = false;
    mediaChanged?.();
  });
  fireEvent(
    chronology,
    new MouseEvent("pointermove", { bubbles: true, clientX: 100, clientY: 100 }),
  );
  expect(callbacks.size).toBe(1);
  act(() => {
    motion.matches = true;
    mediaChanged?.();
  });
  expect(callbacks.size).toBe(0);
  unmount();
  expect(motion.removeEventListener).toHaveBeenCalledWith("change", mediaChanged);
});

it("keeps touch selection available without scheduling hover effects", () => {
  const { onSelect } = rail([moment("first")]);
  const chronology = screen.getByRole("region", { name: "Chronological timeline" });
  const event = new MouseEvent("pointermove", { bubbles: true, clientX: 100, clientY: 100 });
  Object.defineProperty(event, "pointerType", { value: "touch" });
  fireEvent(chronology, event);
  expect(requestAnimationFrame).not.toHaveBeenCalled();
  fireEvent.click(within(chronology).getByRole("button", { name: /Moment first/ }));
  expect(onSelect).toHaveBeenCalledWith("first");
});

it("resets horizontal position for a different page or filter but preserves it on autosave and expansion", () => {
  const props = { calendar, selectedId: null, disabled: false, onSelect: vi.fn() };
  const first = moment("first");
  const { rerender } = render(<TimelineRail {...props} occurrences={[first]} viewKey="all:0" />);
  const chronology = screen.getByRole("region", { name: "Chronological timeline" });
  chronology.scrollLeft = 420;
  rerender(
    <TimelineRail {...props} occurrences={[{ ...first, title: "Saved title" }]} viewKey="all:0" />,
  );
  expect(chronology.scrollLeft).toBe(420);
  rerender(<TimelineRail {...props} occurrences={[first, moment("second")]} viewKey="all:0" />);
  expect(chronology.scrollLeft).toBe(420);
  rerender(<TimelineRail {...props} occurrences={[moment("later", 2)]} viewKey="all:1" />);
  expect(chronology.scrollLeft).toBe(0);
  chronology.scrollLeft = 270;
  rerender(<TimelineRail {...props} occurrences={[moment("matched", 3)]} viewKey="search:0" />);
  expect(chronology.scrollLeft).toBe(0);
});
