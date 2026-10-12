import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { applyWorkspaceNavigation, readWorkspaceNavigation } from "./api";
import { deferred } from "./chapterTestFixtures";
import { navigationRecord, navigationSnapshot } from "./workspaceNavigationTestFixtures";
import type { NavigationSnapshot } from "./workspaceNavigationTypes";
import { useWorkspaceNavigation } from "./useWorkspaceNavigation";

vi.mock("./api", () => ({ applyWorkspaceNavigation: vi.fn(), readWorkspaceNavigation: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readWorkspaceNavigation).mockResolvedValue(navigationSnapshot());
  vi.mocked(applyWorkspaceNavigation).mockResolvedValue(navigationSnapshot());
});

it("serializes visits and reads so a slower result cannot replace a newer visit", async () => {
  const first = deferred<NavigationSnapshot>();
  vi.mocked(applyWorkspaceNavigation)
    .mockReturnValueOnce(first.promise)
    .mockResolvedValueOnce(
      navigationSnapshot({ recents: [navigationRecord("second"), navigationRecord("first")] }),
    );
  const { result } = renderHook(() => useWorkspaceNavigation("project"));
  await waitFor(() => expect(result.current.snapshot).not.toBeNull());
  let a!: Promise<void>, b!: Promise<void>;
  act(() => {
    a = result.current.apply({ kind: "visit", target: { recordKind: "entry", recordId: "first" } });
    b = result.current.apply({
      kind: "visit",
      target: { recordKind: "entry", recordId: "second" },
    });
  });
  await waitFor(() => expect(applyWorkspaceNavigation).toHaveBeenCalledTimes(1));
  await act(async () => {
    first.resolve(navigationSnapshot({ recents: [navigationRecord("first")] }));
    await a;
    await b;
  });
  expect(applyWorkspaceNavigation).toHaveBeenNthCalledWith(2, "project", {
    kind: "visit",
    target: { recordKind: "entry", recordId: "second" },
  });
  expect(result.current.snapshot?.recents.map((r) => r.recordId)).toEqual(["second", "first"]);
  expect(result.current.busy).toBe(false);
});

it("keeps failed pin intent for explicit retry even when a later read succeeds", async () => {
  vi.mocked(applyWorkspaceNavigation).mockRejectedValueOnce(new Error("Disk full"));
  const { result } = renderHook(() => useWorkspaceNavigation("project"));
  await waitFor(() => expect(result.current.snapshot).not.toBeNull());
  const command = {
    kind: "pin" as const,
    target: { recordKind: "entry" as const, recordId: "stable" },
    pinned: true,
  };
  await act(async () => {
    await result.current.apply(command);
    await result.current.refresh();
  });
  expect(result.current.error).toBe("Disk full");
  expect(result.current.snapshot?.pins).toEqual([]);
  vi.mocked(readWorkspaceNavigation).mockRejectedValueOnce(new Error("Read failed too"));
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.error).toBe("Disk full");
  vi.mocked(applyWorkspaceNavigation).mockRejectedValueOnce(new Error("Visit failed too"));
  await act(async () => {
    await result.current.apply({
      kind: "visit",
      target: { recordKind: "entry", recordId: "another" },
    });
  });
  expect(result.current.error).toBe("Disk full");
  vi.mocked(applyWorkspaceNavigation).mockResolvedValue(
    navigationSnapshot({ pins: [navigationRecord("stable", { label: "Renamed" })] }),
  );
  await act(async () => {
    await result.current.retry();
  });
  expect(applyWorkspaceNavigation).toHaveBeenLastCalledWith("project", command);
  expect(result.current.error).toBeNull();
  expect(result.current.snapshot?.pins[0].label).toBe("Renamed");
});

it("can recover an unavailable metadata read without blocking later writes", async () => {
  vi.mocked(readWorkspaceNavigation).mockRejectedValueOnce(new Error("Unavailable"));
  const { result } = renderHook(() => useWorkspaceNavigation("project"));
  await waitFor(() => expect(result.current.error).toBe("Unavailable"));
  await act(async () => {
    await result.current.retry();
  });
  expect(result.current.snapshot).toEqual(navigationSnapshot());
  expect(result.current.error).toBeNull();
});

it.each(["success", "failure"])(
  "ignores an old Project's delayed %s and queued operations",
  async (outcome) => {
    const old = deferred<NavigationSnapshot>();
    vi.mocked(applyWorkspaceNavigation).mockReturnValueOnce(old.promise);
    const { result, rerender } = renderHook(({ id }) => useWorkspaceNavigation(id), {
      initialProps: { id: "old" },
    });
    await waitFor(() => expect(result.current.snapshot).not.toBeNull());
    let a!: Promise<void>, b!: Promise<void>;
    act(() => {
      a = result.current.apply({
        kind: "visit",
        target: { recordKind: "entry", recordId: "old-entry" },
      });
      b = result.current.apply({ kind: "clear_recents" });
    });
    await waitFor(() => expect(applyWorkspaceNavigation).toHaveBeenCalledTimes(1));
    vi.mocked(readWorkspaceNavigation).mockResolvedValue(
      navigationSnapshot({ pins: [navigationRecord("new")] }),
    );
    rerender({ id: "new" });
    await waitFor(() => expect(result.current.snapshot?.pins[0].recordId).toBe("new"));
    await act(async () => {
      if (outcome === "success") old.resolve(navigationSnapshot());
      else old.reject(new Error("Old failure"));
      await a;
      await b;
    });
    expect(result.current.snapshot?.pins[0].recordId).toBe("new");
    expect(result.current.error).toBeNull();
    expect(result.current.busy).toBe(false);
    expect(applyWorkspaceNavigation).toHaveBeenCalledTimes(1);
  },
);
