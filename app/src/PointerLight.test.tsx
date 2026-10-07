import { act, fireEvent, render, screen } from "@testing-library/react";
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
function move(pointerType = "mouse", x = 100, target: EventTarget = document) {
  const event = new Event("pointermove", { bubbles: true });
  Object.assign(event, { pointerType, clientX: x, clientY: 80 });
  target.dispatchEvent(event);
}
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
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

function surfaces() {
  const view = render(
    <>
      <PointerLight />
      <section className="panel" aria-label="First surface">
        <button>Inner target</button>
      </section>
      <aside className="project-sidebar" aria-label="Second surface">
        Sidebar
      </aside>
      <div data-testid="outside">Outside surfaces</div>
    </>,
  );
  const first = screen.getByRole("region", { name: "First surface" });
  const second = screen.getByRole("complementary", { name: "Second surface" });
  vi.spyOn(first, "getBoundingClientRect").mockReturnValue({
    left: 20,
    top: 30,
    width: 300,
    height: 200,
  } as DOMRect);
  vi.spyOn(second, "getBoundingClientRect").mockReturnValue({
    left: 60,
    top: 50,
    width: 240,
    height: 400,
  } as DOMRect);
  return { ...view, first, second };
}

it("lights the nearest surface at local coordinates and clears the previous surface when moving", () => {
  const { frame } = setup();
  const { first, second } = surfaces();
  move("mouse", 100, screen.getByRole("button", { name: "Inner target" }));
  frame();
  expect(first).toHaveAttribute("data-illuminated", "on");
  expect(first.style.getPropertyValue("--surface-pointer-x")).toBe("80px");
  expect(first.style.getPropertyValue("--surface-pointer-y")).toBe("50px");
  expect(second).not.toHaveAttribute("data-illuminated");
  move("mouse", 150, second);
  expect(first).not.toHaveAttribute("data-illuminated");
  frame();
  expect(second).toHaveAttribute("data-illuminated", "on");
  expect(second.style.getPropertyValue("--surface-pointer-x")).toBe("90px");
  expect(second.style.getPropertyValue("--surface-pointer-y")).toBe("30px");
  move("mouse", 180, screen.getByTestId("outside"));
  expect(second).not.toHaveAttribute("data-illuminated");
  frame();
  expect(document.documentElement.dataset.pointerLight).toBe("on");
  expect(document.querySelector("[data-illuminated]")).toBeNull();
});

it("coalesces moves between surfaces into one frame without writing layout dimensions", () => {
  const { frame } = setup();
  const { first, second } = surfaces();
  const write = vi.spyOn(second.style, "setProperty");
  move("mouse", 100, first);
  move("mouse", 180, second);
  move("mouse", 220, second);
  expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
  frame();
  expect(first).not.toHaveAttribute("data-illuminated");
  expect(first.getBoundingClientRect).not.toHaveBeenCalled();
  expect(second.getBoundingClientRect).toHaveBeenCalledTimes(1);
  expect(second.style.getPropertyValue("--surface-pointer-x")).toBe("160px");
  expect(document.documentElement.style.getPropertyValue("--pointer-x")).toBe("220px");
  expect(write.mock.calls.map(([name]) => name)).toEqual([
    "--surface-pointer-x",
    "--surface-pointer-y",
  ]);
  expect(first.style.width).toBe("");
  expect(second.style.width).toBe("");
  expect(second.style.height).toBe("");
});

it.each(["touch", "pen"])(
  "clears illuminated surfaces and cancels pending mouse updates for %s",
  (pointerType) => {
    const { frame } = setup();
    const { first } = surfaces();
    move("mouse", 100, first);
    frame();
    move("mouse", 120, first);
    move(pointerType, 120, first);
    frame();
    expect(first).not.toHaveAttribute("data-illuminated");
    expect(document.documentElement.dataset.pointerLight).toBeUndefined();
  },
);

it.each(["(prefers-reduced-motion: reduce)", "(forced-colors: active)"])(
  "clears surface illumination and pending work when %s activates",
  (query) => {
    const { queries, frame } = setup();
    const { first } = surfaces();
    move("mouse", 100, first);
    frame();
    move("mouse", 120, first);
    const media = queries.get(query)!;
    media.matches = true;
    media.change();
    frame();
    expect(first).not.toHaveAttribute("data-illuminated");
    move("mouse", 150, first);
    frame();
    expect(first).not.toHaveAttribute("data-illuminated");
    expect(document.documentElement.dataset.pointerLight).toBeUndefined();
  },
);

it("keeps illumination within the document and clears it on window exit, blur, hidden state and unmount", () => {
  const { frame } = setup();
  const { first, second, unmount } = surfaces();
  move("mouse", 100, first);
  frame();
  const innerLeave = new Event("pointerout", { bubbles: true });
  Object.assign(innerLeave, { relatedTarget: second });
  first.dispatchEvent(innerLeave);
  expect(first).toHaveAttribute("data-illuminated", "on");
  document.dispatchEvent(new Event("pointerout"));
  expect(first).not.toHaveAttribute("data-illuminated");
  move("mouse", 100, first);
  frame();
  fireEvent(window, new Event("blur"));
  expect(first).not.toHaveAttribute("data-illuminated");
  move("mouse", 100, first);
  frame();
  vi.spyOn(document, "hidden", "get").mockReturnValue(true);
  document.dispatchEvent(new Event("visibilitychange"));
  expect(first).not.toHaveAttribute("data-illuminated");
  move("mouse", 100, first);
  frame();
  move("mouse", 120, first);
  unmount();
  frame();
  expect(first).not.toHaveAttribute("data-illuminated");
  expect(document.documentElement.style.getPropertyValue("--pointer-x")).toBe("");
});
