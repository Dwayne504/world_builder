import { act, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { PointerLight } from "./PointerLight";

function setup() {
  const queries = new Map<string, { matches: boolean; change: () => void }>();
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => {
      const media = {
        // A WebView/hybrid device may report no fine pointer or hover.
        matches: false,
        change: () => {},
        addEventListener: (_event: string, change: () => void) => {
          media.change = change;
        },
        removeEventListener: vi.fn(),
      };
      queries.set(query, media);
      return media;
    }),
  );
  let callback: FrameRequestCallback | undefined;
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn((next: FrameRequestCallback) => {
      callback = next;
      return 1;
    }),
  );
  vi.stubGlobal(
    "cancelAnimationFrame",
    vi.fn(() => {
      callback = undefined;
    }),
  );
  return {
    queries,
    frame: () => {
      act(() => {
        const fn = callback;
        callback = undefined;
        fn?.(0);
      });
    },
  };
}
function move(pointerType = "mouse", x = 100) {
  const event = new Event("pointermove");
  Object.assign(event, { pointerType, clientX: x, clientY: 80 });
  document.dispatchEvent(event);
}
afterEach(() => vi.unstubAllGlobals());
it("responds to a real mouse even without hover media, coalesces updates, excludes touch and cleans up", () => {
  const { frame } = setup();
  const view = render(<PointerLight />);
  move();
  move("mouse", 120);
  expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
  frame();
  expect(document.documentElement.dataset.pointerLight).toBe("on");
  expect(document.documentElement.style.getPropertyValue("--pointer-x")).toBe("120px");
  expect(view.container.querySelector(".pointer-light")).toHaveAttribute("aria-hidden", "true");
  move("touch");
  expect(document.documentElement.dataset.pointerLight).toBeUndefined();
  move();
  frame();
  document.dispatchEvent(new Event("pointerout"));
  expect(document.documentElement.dataset.pointerLight).toBeUndefined();
  move();
  view.unmount();
  frame();
  expect(document.documentElement.dataset.pointerLight).toBeUndefined();
  expect(document.documentElement.style.getPropertyValue("--pointer-x")).toBe("");
});
it.each(["(prefers-reduced-motion: reduce)", "(forced-colors: active)"])(
  "disables illumination when %s changes",
  (query) => {
    const { queries, frame } = setup();
    render(<PointerLight />);
    move();
    frame();
    const media = queries.get(query)!;
    media.matches = true;
    media.change();
    expect(document.documentElement.dataset.pointerLight).toBeUndefined();
    move();
    frame();
    expect(document.documentElement.dataset.pointerLight).toBeUndefined();
    media.matches = false;
    media.change();
    move();
    frame();
    expect(document.documentElement.dataset.pointerLight).toBe("on");
  },
);
