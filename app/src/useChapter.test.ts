import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { applyStory, readChapter } from "./api";
import { useChapter } from "./useChapter";
import { chapterFixture, deferred, textDocument } from "./chapterTestFixtures";
import { readChapterDraft } from "./chapterRecovery";
import type { ChapterSnapshot } from "./storyTypes";
vi.mock("./api", () => ({ applyStory: vi.fn(), readChapter: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
});
afterEach(() => vi.useRealTimers());

it("debounces writing, retains newer typing during an acknowledgement, and drains on close", async () => {
  const initial = chapterFixture();
  const pending = deferred<ChapterSnapshot>();
  vi.mocked(applyStory)
    .mockReturnValueOnce(pending.promise)
    .mockResolvedValue({ ...initial, globalRevision: 5 });
  const { result } = renderHook(() => useChapter("project", initial, vi.fn()));
  act(() => result.current.changeDocument("manuscript", textDocument("First draft")));
  expect(result.current.state).toBe("dirty");
  expect(readChapterDraft("project", initial)?.draft.documents.manuscript).toEqual(
    textDocument("First draft"),
  );
  await waitFor(() => {
    expect(applyStory).toHaveBeenCalledTimes(1);
    expect(result.current.state).toBe("saving");
  });
  act(() => result.current.changeDocument("manuscript", textDocument("First draft, more writing")));
  let flush!: Promise<unknown>;
  act(() => {
    flush = result.current.flush();
  });
  await act(async () => {
    pending.resolve({ ...initial, globalRevision: 4 });
    await flush;
  });
  expect(applyStory).toHaveBeenLastCalledWith(
    "project",
    4,
    expect.objectContaining({
      documents: [
        {
          area: "manuscript",
          schemaVersion: 1,
          content: textDocument("First draft, more writing"),
        },
      ],
    }),
  );
  expect(result.current.state).toBe("saved");
  expect(readChapterDraft("project", initial)).toBeNull();
});
it("flushes immediately without waiting for debounce and preserves independent areas", async () => {
  const initial = chapterFixture();
  vi.mocked(applyStory).mockResolvedValue({ ...initial, globalRevision: 4 });
  const { result } = renderHook(() => useChapter("project", initial, vi.fn()));
  act(() => {
    result.current.changeTitle("Opening");
    result.current.changeDocument("notes", textDocument("A private note"));
  });
  await act(async () => {
    await result.current.flush();
  });
  expect(applyStory).toHaveBeenCalledWith("project", 3, {
    kind: "save",
    chapterId: "chapter",
    title: "Opening",
    documents: [{ area: "notes", schemaVersion: 1, content: textDocument("A private note") }],
  });
});
it("keeps failed writing and a recovery copy without automatic retries or rebasing", async () => {
  vi.useFakeTimers();
  const initial = chapterFixture();
  vi.mocked(applyStory).mockRejectedValueOnce(new Error("Stale revision"));
  const { result } = renderHook(() => useChapter("project", initial, vi.fn()));
  act(() => result.current.changeDocument("manuscript", textDocument("Keep this")));
  await act(async () => {
    await result.current.flush();
  });
  expect(result.current.state).toBe("failed");
  act(() => result.current.changeDocument("manuscript", textDocument("Keep this too")));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
  });
  expect(applyStory).toHaveBeenCalledTimes(1);
  expect(readChapterDraft("project", initial)?.draft.documents.manuscript).toEqual(
    textDocument("Keep this too"),
  );
  vi.mocked(applyStory).mockResolvedValue({ ...initial, globalRevision: 4 });
  await act(async () => {
    await result.current.flush();
  });
  expect(applyStory).toHaveBeenLastCalledWith("project", 3, expect.anything());
  expect(result.current.state).toBe("saved");
});
it("does not claim the emergency buffer succeeded when local storage is unavailable", () => {
  const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("Quota exceeded");
  });
  const { result, unmount } = renderHook(() => useChapter("project", chapterFixture(), vi.fn()));
  act(() => result.current.changeTitle("Preserve in memory"));
  expect(result.current.recoveryAvailable).toBe(false);
  expect(result.current.draft.title).toBe("Preserve in memory");
  expect(result.current.state).toBe("dirty");
  unmount();
  spy.mockRestore();
});

it("requires explicit saved-version review before retrying against a newer revision", async () => {
  const initial = chapterFixture();
  vi.mocked(applyStory).mockRejectedValueOnce(new Error("Stale revision"));
  vi.mocked(readChapter).mockResolvedValue({ ...initial, globalRevision: 9 });
  const { result } = renderHook(() => useChapter("project", initial, vi.fn()));
  act(() => result.current.changeDocument("notes", textDocument("Keep my notes")));
  await act(async () => {
    await result.current.flush();
  });
  let held: Awaited<ReturnType<typeof result.current.reloadSaved>>;
  await act(async () => {
    held = await result.current.reloadSaved();
  });
  expect(result.current.snapshot.globalRevision).toBe(9);
  expect(applyStory).toHaveBeenCalledTimes(1);
  expect(readChapterDraft("project", initial)?.draft.documents.notes).toEqual(
    textDocument("Keep my notes"),
  );
  vi.mocked(applyStory).mockResolvedValue({ ...initial, globalRevision: 10 });
  act(() => result.current.restoreDraft(held!.draft));
  await act(async () => {
    await result.current.flush();
  });
  expect(applyStory).toHaveBeenLastCalledWith(
    "project",
    9,
    expect.objectContaining({ kind: "save" }),
  );
});
