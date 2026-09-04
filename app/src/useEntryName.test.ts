import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Entry } from "./types";

const updateEntryNameMock = vi.fn();
vi.mock("./api", () => ({
  updateEntryName: (...args: unknown[]) => updateEntryNameMock(...args),
}));

import { useEntryName } from "./useEntryName";

const entry: Entry = {
  id: "entry-id",
  categoryId: "category-id",
  typeId: null,
  authoredName: "Thron",
  displayName: "Thron",
  revision: 1,
  globalRevision: 2,
};

describe("useEntryName", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    updateEntryNameMock.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  it("debounces typing and displays Saved only after commit acknowledgement", async () => {
    let resolveSave!: (value: Entry) => void;
    updateEntryNameMock.mockReturnValue(
      new Promise((resolve) => {
        resolveSave = resolve;
      }),
    );
    const { result } = renderHook(() => useEntryName("project-id", entry));
    act(() => result.current.onChangeDraft("Thron II"));
    expect(result.current.saveState).toBe("dirty");
    act(() => vi.advanceTimersByTime(499));
    expect(updateEntryNameMock).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));
    expect(result.current.saveState).toBe("saving");
    expect(result.current.saveState).not.toBe("saved");
    await act(async () => {
      resolveSave({ ...entry, authoredName: "Thron II", displayName: "Thron II", revision: 2 });
      await Promise.resolve();
    });
    expect(result.current.saveState).toBe("saved");
  });

  it("preserves a newer draft when an older save resolves", async () => {
    let resolveSave!: (value: Entry) => void;
    updateEntryNameMock.mockReturnValue(
      new Promise((resolve) => {
        resolveSave = resolve;
      }),
    );
    const { result } = renderHook(() => useEntryName("project-id", entry));
    act(() => {
      result.current.onChangeDraft("A");
      vi.advanceTimersByTime(500);
      result.current.onChangeDraft("B");
    });
    await act(async () => {
      resolveSave({ ...entry, authoredName: "A", displayName: "A", revision: 2 });
      await Promise.resolve();
    });
    expect(result.current.draftName).toBe("B");
    expect(result.current.saveState).toBe("dirty");
  });

  it("keeps failed writes visibly unsaved", async () => {
    updateEntryNameMock.mockRejectedValue(new Error("disk full"));
    const { result } = renderHook(() => useEntryName("project-id", entry));
    act(() => result.current.onChangeDraft("Broken"));
    await act(async () => {
      vi.advanceTimersByTime(500);
      await Promise.resolve();
    });
    expect(result.current.saveState).toBe("failed");
    expect(result.current.draftName).toBe("Broken");
  });

  it("does not erase a newer name draft when a structure response is merged", () => {
    const { result } = renderHook(() => useEntryName("project-id", entry));
    act(() => {
      result.current.onChangeDraft("Newer draft");
      result.current.replaceEntry({
        ...entry,
        categoryId: "new-category",
        revision: 2,
        globalRevision: 3,
      });
    });
    expect(result.current.draftName).toBe("Newer draft");
    expect(result.current.saveState).toBe("dirty");
    expect(result.current.entry.categoryId).toBe("new-category");
  });
});
