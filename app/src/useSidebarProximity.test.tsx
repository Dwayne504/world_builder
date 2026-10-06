import { useRef } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useSidebarProximity } from "./useSidebarProximity";

type Media = {
  matches: boolean;
  change: () => void;
  removeEventListener: ReturnType<typeof vi.fn>;
};
let media: Map<string, Media>;
let frames: Map<number, FrameRequestCallback>;

beforeEach(() => {
  media = new Map();
  frames = new Map();
  let frameId = 0;
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => {
      const value = {
        matches: false,
        change: () => {},
        addEventListener: (_event: string, listener: () => void) => {
          value.change = listener;
        },
        removeEventListener: vi.fn(),
      };
      media.set(query, value);
      return value;
    }),
  );
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn((callback: FrameRequestCallback) => {
      frames.set(++frameId, callback);
      return frameId;
    }),
  );
  vi.stubGlobal(
    "cancelAnimationFrame",
    vi.fn((id: number) => {
      frames.delete(id);
    }),
  );
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function Harness() {
  const ref = useRef<HTMLElement>(null);
  useSidebarProximity(ref);
  return (
    <aside ref={ref} aria-label="Sidebar">
      <button className="sidebar-destination">
        <span>First destination</span>
      </button>
      <button className="sidebar-destination">
        <span>Second destination</span>
      </button>
      <button className="sidebar-destination" disabled>
        Disabled destination
      </button>
    </aside>
  );
}
function setup() {
  const view = render(<Harness />);
  const sidebar = screen.getByRole("complementary", { name: "Sidebar" });
  const first = screen.getByRole("button", { name: "First destination" });
  const second = screen.getByRole("button", { name: "Second destination" });
  const disabled = screen.getByRole("button", { name: "Disabled destination" });
  const bounds = (top: number) => ({
    top,
    height: 40,
    left: 0,
    width: 220,
    bottom: top + 40,
    right: 220,
    x: 0,
    y: top,
    toJSON: () => ({}),
  });
  vi.spyOn(first, "getBoundingClientRect").mockReturnValue(bounds(80));
  vi.spyOn(second, "getBoundingClientRect").mockReturnValue(bounds(120));
  vi.spyOn(disabled, "getBoundingClientRect").mockReturnValue(bounds(80));
  return { ...view, sidebar, first, second, disabled };
}
function move(target: Element, pointerType = "mouse", y = 100) {
  const event = new Event("pointermove", { bubbles: true });
  Object.assign(event, { pointerType, clientX: 100, clientY: y });
  fireEvent(target, event);
}
function frame() {
  act(() => {
    const callbacks = [...frames.values()];
    frames.clear();
    callbacks.forEach((callback) => callback(0));
  });
}

it("uses actual mouse events on hybrid devices and coalesces to the latest pointer position without changing hit areas", () => {
  const { sidebar, first, second, disabled } = setup();
  const write = vi.spyOn(first.style, "setProperty");
  move(first.querySelector("span")!, "mouse", 125);
  move(sidebar, "mouse", 100);
  expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
  expect(first.getBoundingClientRect).not.toHaveBeenCalled();
  frame();
  expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("1.000");
  expect(Number(second.style.getPropertyValue("--sidebar-nearness"))).toBeCloseTo(1 - 40 / 76, 3);
  expect(disabled.style.getPropertyValue("--sidebar-nearness")).toBe("0.000");
  expect(write.mock.calls.every(([name]) => name === "--sidebar-nearness")).toBe(true);
  expect(first.style.width).toBe("");
  expect(first.style.height).toBe("");
  expect(sidebar.style.gridTemplateColumns).toBe("");
  expect(window.matchMedia).not.toHaveBeenCalledWith("(hover: hover)");
});

it.each(["touch", "pen"])("clears proximity and pending frames for %s input", (pointerType) => {
  const { sidebar, first } = setup();
  move(sidebar);
  frame();
  expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("1.000");
  move(sidebar);
  expect(frames.size).toBe(1);
  move(sidebar, pointerType);
  expect(frames.size).toBe(0);
  frame();
  expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
});

it.each(["(prefers-reduced-motion: reduce)", "(forced-colors: active)"])(
  "clears and suppresses proximity while %s is active",
  (query) => {
    const { sidebar, first } = setup();
    move(sidebar);
    frame();
    move(sidebar);
    const preference = media.get(query)!;
    preference.matches = true;
    act(() => preference.change());
    expect(frames.size).toBe(0);
    expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
    move(sidebar);
    frame();
    expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
    preference.matches = false;
    act(() => preference.change());
    move(sidebar);
    frame();
    expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("1.000");
  },
);

it.each(["pointerleave", "scroll", "blur"])(
  "clears painted state and queued work on %s",
  (event) => {
    const { sidebar, first } = setup();
    move(sidebar);
    frame();
    move(sidebar);
    fireEvent(event === "blur" ? window : sidebar, new Event(event));
    expect(frames.size).toBe(0);
    expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
    frame();
    expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
  },
);

it("removes listeners, visual state and pending work when the sidebar unmounts", () => {
  const { sidebar, first, unmount } = setup();
  move(sidebar);
  frame();
  move(sidebar);
  unmount();
  expect(frames.size).toBe(0);
  expect(first.style.getPropertyValue("--sidebar-nearness")).toBe("");
  const calls = vi.mocked(requestAnimationFrame).mock.calls.length;
  move(sidebar);
  frame();
  expect(requestAnimationFrame).toHaveBeenCalledTimes(calls);
  for (const preference of media.values())
    expect(preference.removeEventListener).toHaveBeenCalledWith("change", expect.any(Function));
});
