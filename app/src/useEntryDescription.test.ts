import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { readEntryDescription, saveEntryDescription } from "./api";
import { useEntryDescription } from "./useEntryDescription";
import { readEntryDescriptionDraft } from "./entryDescriptionRecovery";
import { descriptionFixture } from "./entryDescriptionTestFixtures";
import { deferred, textDocument } from "./chapterTestFixtures";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";

vi.mock("./api", () => ({ readEntryDescription: vi.fn(), saveEntryDescription: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  vi.mocked(readEntryDescription).mockResolvedValue(descriptionFixture());
});
afterEach(() => vi.useRealTimers());

async function show() {
  const revision = vi.fn();
  const latestRevision = vi.fn(() => 3);
  const hook = renderHook(() => useEntryDescription("project", "entry", revision, latestRevision));
  await waitFor(() => expect(hook.result.current.snapshot).not.toBeNull());
  return { ...hook, revision, latestRevision };
}

it("does not create a document for an empty Entry and uses acknowledged companion revisions", async () => {
  const { result, latestRevision, revision } = await show();
  await act(async () => {
    expect(await result.current.flush()).toEqual({ kind: "no-op" });
  });
  expect(saveEntryDescription).not.toHaveBeenCalled();
  latestRevision.mockReturnValue(9);
  vi.mocked(saveEntryDescription).mockResolvedValue({
    ...descriptionFixture("Lore"),
    globalRevision: 10,
  });
  act(() => result.current.change(textDocument("Lore")));
  await act(async () => {
    await result.current.flush();
  });
  expect(saveEntryDescription).toHaveBeenCalledWith(
    "project",
    "entry",
    9,
    null,
    1,
    textDocument("Lore"),
  );
  expect(revision).toHaveBeenLastCalledWith(10);
  expect(result.current.state).toBe("saved");
});

it("debounces, retains typing during acknowledgement and drains all writing before close", async () => {
  const pending = deferred<EntryDescriptionSnapshot>();
  vi.mocked(saveEntryDescription)
    .mockReturnValueOnce(pending.promise)
    .mockResolvedValue({ ...descriptionFixture("More lore"), globalRevision: 5 });
  const { result } = await show();
  act(() => result.current.change(textDocument("Lore")));
  await waitFor(() => expect(saveEntryDescription).toHaveBeenCalledTimes(1));
  act(() => result.current.change(textDocument("More lore")));
  let flush!: Promise<unknown>;
  act(() => {
    flush = result.current.flush();
  });
  await act(async () => {
    pending.resolve({ ...descriptionFixture("Lore"), globalRevision: 4 });
    await flush;
  });
  expect(saveEntryDescription).toHaveBeenLastCalledWith(
    "project",
    "entry",
    4,
    1,
    1,
    textDocument("More lore"),
  );
  expect(result.current.state).toBe("saved");
  expect(readEntryDescriptionDraft("project", descriptionFixture())).toBeNull();
});

it("preserves stale/failed drafts without automatic retry, blur retry or silent rebasing", async () => {
  const { result, latestRevision } = await show();
  vi.useFakeTimers();
  vi.mocked(saveEntryDescription).mockRejectedValue(new Error("Stale revision"));
  act(() => result.current.change(textDocument("Keep this")));
  await act(async () => {
    await result.current.flush();
  });
  latestRevision.mockReturnValue(8);
  act(() => result.current.change(textDocument("Keep this too")));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2000);
    await result.current.flush();
  });
  expect(saveEntryDescription).toHaveBeenCalledTimes(1);
  expect(readEntryDescriptionDraft("project", descriptionFixture())?.content).toEqual(
    textDocument("Keep this too"),
  );
  await act(async () => {
    await result.current.retry();
  });
  expect(saveEntryDescription).toHaveBeenLastCalledWith(
    "project",
    "entry",
    3,
    null,
    1,
    textDocument("Keep this too"),
  );
  vi.mocked(readEntryDescription).mockResolvedValue({
    ...descriptionFixture("Other saved writing"),
    globalRevision: 9,
  });
  await act(async () => {
    await result.current.reviewSaved();
  });
  expect(result.current.recovery?.content).toEqual(textDocument("Keep this too"));
  await act(async () => {
    expect(await result.current.flush()).toEqual({ kind: "failed" });
  });
  expect(saveEntryDescription).toHaveBeenCalledTimes(2);
  vi.mocked(saveEntryDescription).mockResolvedValue({
    ...descriptionFixture("Keep this too"),
    globalRevision: 10,
  });
  act(() => result.current.useRecovered());
  await act(async () => {
    await result.current.flush();
  });
  expect(saveEntryDescription).toHaveBeenLastCalledWith(
    "project",
    "entry",
    9,
    1,
    1,
    textDocument("Keep this too"),
  );
});

it("keeps drafts in memory and reports when local emergency recovery storage fails", async () => {
  const { result } = await show();
  const storage = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("Quota exceeded");
  });
  act(() => result.current.change(textDocument("Keep this in memory")));
  expect(result.current.draft).toEqual(textDocument("Keep this in memory"));
  expect(result.current.recoveryAvailable).toBe(false);
  storage.mockRestore();
});

it("retains the draft and failed state when loading a saved version also fails", async () => {
  const { result } = await show();
  vi.mocked(saveEntryDescription).mockRejectedValue(new Error("Disk unavailable"));
  act(() => result.current.change(textDocument("Unsaved lore")));
  await act(async () => {
    await result.current.flush();
  });
  vi.mocked(readEntryDescription).mockRejectedValue(new Error("Still unavailable"));
  await act(async () => {
    await result.current.reviewSaved();
  });
  expect(result.current.draft).toEqual(textDocument("Unsaved lore"));
  expect(result.current.state).toBe("failed");
  expect(result.current.error).toBe("Still unavailable");
});
it("ignores a saved-version read returned after leaving the Entry", async () => {
  const { result, revision, unmount } = await show();
  const pending = deferred<EntryDescriptionSnapshot>();
  vi.mocked(readEntryDescription).mockReturnValue(pending.promise);
  let review!: Promise<void>;
  act(() => {
    review = result.current.reviewSaved();
  });
  unmount();
  await act(async () => {
    pending.resolve({ ...descriptionFixture("Later read"), globalRevision: 8 });
    await review;
  });
  expect(revision).toHaveBeenCalledTimes(1);
  expect(revision).toHaveBeenLastCalledWith(3);
});
