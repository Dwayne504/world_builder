import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { createBackup, getAutomaticBackupStatus, setAutomaticBackupsEnabled } from "./api";
import { useAutomaticBackups } from "./useAutomaticBackups";
import { automaticBackupFixture, backupPreferences } from "./automaticBackupTestFixtures";
import { deferred } from "./chapterTestFixtures";
import type { AutomaticBackupStatus } from "./automaticBackupTypes";

vi.mock("./api", () => ({
  createBackup: vi.fn(),
  getAutomaticBackupStatus: vi.fn(),
  setAutomaticBackupsEnabled: vi.fn(),
}));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getAutomaticBackupStatus).mockResolvedValue(automaticBackupFixture());
});
afterEach(() => vi.useRealTimers());

it("only polls native status, never initiates backups from its timer, and cleans up on unmount", async () => {
  vi.useFakeTimers();
  const { result, unmount } = renderHook(() => useAutomaticBackups("project"));
  await act(async () => {
    await Promise.resolve();
  });
  expect(result.current.status?.enabled).toBe(true);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(90_000);
  });
  expect(getAutomaticBackupStatus).toHaveBeenCalledTimes(4);
  expect(createBackup).not.toHaveBeenCalled();
  unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(60_000);
  });
  expect(getAutomaticBackupStatus).toHaveBeenCalledTimes(4);
});

it("does not overlap slow status reads and refreshes when the window regains focus", async () => {
  vi.useFakeTimers();
  const pending = deferred<AutomaticBackupStatus>();
  vi.mocked(getAutomaticBackupStatus).mockReturnValueOnce(pending.promise);
  renderHook(() => useAutomaticBackups("project"));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(60_000);
  });
  expect(getAutomaticBackupStatus).toHaveBeenCalledTimes(1);
  await act(async () => {
    pending.resolve(automaticBackupFixture());
  });
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  expect(getAutomaticBackupStatus).toHaveBeenCalledTimes(2);
});

it("reports status failures without losing the previous last successful backup", async () => {
  const previous = automaticBackupFixture({
    lastSuccessAt: "2026-01-02T12:00:00Z",
    lastSuccessRevision: 8,
  });
  vi.mocked(getAutomaticBackupStatus)
    .mockResolvedValueOnce(previous)
    .mockRejectedValueOnce(new Error("Status unavailable"));
  const { result } = renderHook(() => useAutomaticBackups("project"));
  await waitFor(() => expect(result.current.status).toEqual(previous));
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.status).toEqual(previous);
  expect(result.current.readError).toBe("Status unavailable");
  vi.mocked(getAutomaticBackupStatus).mockResolvedValue(automaticBackupFixture({ enabled: false }));
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.readError).toBeNull();
  expect(result.current.status?.enabled).toBe(false);
});

it("applies the setting only after acknowledgement and rejects an earlier stale status read", async () => {
  const stale = deferred<AutomaticBackupStatus>();
  vi.mocked(getAutomaticBackupStatus)
    .mockReturnValueOnce(stale.promise)
    .mockResolvedValue(automaticBackupFixture({ enabled: false, nextDueAt: null }));
  vi.mocked(setAutomaticBackupsEnabled).mockResolvedValue(backupPreferences(false));
  const { result } = renderHook(() => useAutomaticBackups("project"));
  await act(async () => {
    await result.current.setEnabled(false);
  });
  expect(setAutomaticBackupsEnabled).toHaveBeenCalledWith(false);
  expect(result.current.status?.enabled).toBe(false);
  await act(async () => {
    stale.resolve(automaticBackupFixture({ enabled: true }));
  });
  expect(result.current.status?.enabled).toBe(false);
});

it("retains the acknowledged setting when a preference change fails", async () => {
  vi.mocked(setAutomaticBackupsEnabled).mockRejectedValue(new Error("Preferences are newer"));
  const { result } = renderHook(() => useAutomaticBackups("project"));
  await waitFor(() => expect(result.current.status?.enabled).toBe(true));
  await act(async () => {
    await result.current.setEnabled(false);
  });
  expect(result.current.status?.enabled).toBe(true);
  expect(result.current.actionError).toBe("Preferences are newer");
  expect(result.current.changing).toBe(false);
});

it("discards a delayed response belonging to a previous Project", async () => {
  const old = deferred<AutomaticBackupStatus>();
  vi.mocked(getAutomaticBackupStatus)
    .mockReturnValueOnce(old.promise)
    .mockResolvedValue(automaticBackupFixture({ backupDirectory: "/second" }));
  const { result, rerender } = renderHook(({ id }) => useAutomaticBackups(id), {
    initialProps: { id: "first" },
  });
  rerender({ id: "second" });
  await waitFor(() => expect(result.current.status?.backupDirectory).toBe("/second"));
  await act(async () => {
    old.resolve(automaticBackupFixture({ backupDirectory: "/first" }));
  });
  expect(result.current.status?.backupDirectory).toBe("/second");
});
it.each(["success", "failure"])(
  "does not apply a previous Project's delayed preference %s to a new status view",
  async (outcome) => {
    const setting = deferred<ReturnType<typeof backupPreferences>>();
    vi.mocked(setAutomaticBackupsEnabled).mockReturnValueOnce(setting.promise);
    const { result, rerender } = renderHook(({ id }) => useAutomaticBackups(id), {
      initialProps: { id: "first" },
    });
    await waitFor(() => expect(result.current.status).not.toBeNull());
    let update!: Promise<void>;
    act(() => {
      update = result.current.setEnabled(false);
    });
    vi.mocked(getAutomaticBackupStatus).mockResolvedValue(
      automaticBackupFixture({ backupDirectory: "/new-project" }),
    );
    rerender({ id: "second" });
    await waitFor(() => expect(result.current.status?.backupDirectory).toBe("/new-project"));
    await act(async () => {
      if (outcome === "success") setting.resolve(backupPreferences(false));
      else setting.reject(new Error("Previous Project's pending preference failed"));
      await update;
    });
    expect(result.current.status?.backupDirectory).toBe("/new-project");
    expect(result.current.status?.enabled).toBe(true);
    expect(result.current.actionError).toBeNull();
  },
);
