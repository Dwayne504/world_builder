import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const renameProjectMock = vi.fn();
vi.mock("./api", () => ({
  renameProject: (...args: unknown[]) => renameProjectMock(...args),
}));

import { useProjectRename } from "./useProjectRename";
import type { SubmitOutcome } from "./useProjectRename";
import type { ProjectSummary } from "./types";

function makeProject(overrides: Partial<ProjectSummary> = {}): ProjectSummary {
  return {
    projectId: "0198c000-0000-7000-8000-000000000000",
    workingName: "Tortuga",
    revision: 0,
    packagePath: "/tmp/Tortuga.wcproj",
    formatVersion: 1,
    schemaVersion: 1,
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("useProjectRename", () => {
  beforeEach(() => renameProjectMock.mockReset());

  it("transitions dirty -> saving -> saved on a successful rename", async () => {
    const project = makeProject();
    let resolveRename: (value: ProjectSummary) => void = () => {};
    renameProjectMock.mockReturnValueOnce(
      new Promise<ProjectSummary>((resolve) => {
        resolveRename = resolve;
      }),
    );

    const { result } = renderHook(() => useProjectRename(project));
    expect(result.current.saveState).toBe("saved");

    act(() => result.current.onChangeDraft("Tortuga Prime"));
    expect(result.current.saveState).toBe("dirty");

    let submitPromise!: Promise<SubmitOutcome>;
    act(() => {
      submitPromise = result.current.submit();
    });
    expect(result.current.saveState).toBe("saving");

    act(() => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
    });
    await act(async () => {
      await submitPromise;
    });

    expect(result.current.saveState).toBe("saved");
    expect(result.current.committedName).toBe("Tortuga Prime");
    expect(result.current.revision).toBe(1);
  });

  it("does not report Saved when the save fails, and keeps the dirty draft", async () => {
    const project = makeProject();
    renameProjectMock.mockRejectedValueOnce(new Error("disk is full"));

    const { result } = renderHook(() => useProjectRename(project));
    act(() => result.current.onChangeDraft("Tortuga Prime"));

    await act(async () => {
      await result.current.submit();
    });

    expect(result.current.saveState).toBe("failed");
    expect(result.current.saveState).not.toBe("saved");
    expect(result.current.draftName).toBe("Tortuga Prime");
    expect(result.current.errorMessage).toMatch(/disk is full/);
    expect(result.current.hasUnsavedWork).toBe(true);
  });

  it("keeps the Project ID stable across a rename", async () => {
    const project = makeProject();
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });

    const { result } = renderHook(() => useProjectRename(project));
    act(() => result.current.onChangeDraft("Tortuga Prime"));
    await act(async () => {
      await result.current.submit();
    });

    // The hook never exposes a way to change projectId; it is passed
    // through unchanged from the original summary regardless of rename
    // outcome.
    expect(project.projectId).toBe("0198c000-0000-7000-8000-000000000000");
    await waitFor(() => expect(result.current.committedName).toBe("Tortuga Prime"));
  });

  it("reports hasUnsavedWork while dirty or failed, blocking a silent close", async () => {
    const project = makeProject();
    const { result } = renderHook(() => useProjectRename(project));
    expect(result.current.hasUnsavedWork).toBe(false);

    act(() => result.current.onChangeDraft("Changed"));
    expect(result.current.hasUnsavedWork).toBe(true);

    renameProjectMock.mockRejectedValueOnce(new Error("offline"));
    await act(async () => {
      await result.current.submit();
    });
    expect(result.current.hasUnsavedWork).toBe(true);
  });

  it("retains a newer draft after an older save succeeds and advances its revision", async () => {
    const project = makeProject();
    let resolveRename!: (value: ProjectSummary) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise<ProjectSummary>((resolve) => {
        resolveRename = resolve;
      }),
    );
    const { result } = renderHook(() => useProjectRename(project));
    act(() => result.current.onChangeDraft("A"));
    let saving!: Promise<SubmitOutcome>;
    act(() => {
      saving = result.current.submit();
    });
    act(() => result.current.onChangeDraft("B"));
    act(() => resolveRename({ ...project, workingName: "A", revision: 1 }));
    await act(async () => {
      await saving;
    });
    expect(result.current.committedName).toBe("A");
    expect(result.current.revision).toBe(1);
    expect(result.current.draftName).toBe("B");
    expect(result.current.saveState).toBe("dirty");
  });

  it("deduplicates repeated submit calls while saving", async () => {
    const project = makeProject();
    let resolveRename!: (value: ProjectSummary) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise<ProjectSummary>((resolve) => {
        resolveRename = resolve;
      }),
    );
    const { result } = renderHook(() => useProjectRename(project));
    act(() => result.current.onChangeDraft("A"));
    let first!: Promise<SubmitOutcome>;
    let second!: Promise<SubmitOutcome>;
    act(() => {
      first = result.current.submit();
      second = result.current.submit();
    });
    expect(second).toBe(first);
    expect(renameProjectMock).toHaveBeenCalledTimes(1);
    act(() => resolveRename({ ...project, workingName: "A", revision: 1 }));
    await act(async () => {
      await first;
    });
  });

  it("retains a newer draft when the older save fails", async () => {
    let rejectRename!: (reason: Error) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectRename = reject;
      }),
    );
    const { result } = renderHook(() => useProjectRename(makeProject()));
    act(() => result.current.onChangeDraft("A"));
    let saving!: Promise<SubmitOutcome>;
    act(() => {
      saving = result.current.submit();
      result.current.onChangeDraft("B");
    });
    act(() => rejectRename(new Error("disk full")));
    await act(async () => {
      await saving;
    });
    expect(result.current.draftName).toBe("B");
    expect(result.current.saveState).toBe("failed");
  });

  it("resolves submit() with an explicit outcome control-flow callers can branch on", async () => {
    const project = makeProject();

    // Nothing to save: resolves immediately with "no-op", not "committed".
    const { result: idle } = renderHook(() => useProjectRename(project));
    let idleOutcome!: SubmitOutcome;
    await act(async () => {
      idleOutcome = await idle.current.submit();
    });
    expect(idleOutcome).toEqual({ kind: "no-op" });

    // The currently displayed draft commits successfully: "committed".
    renameProjectMock.mockResolvedValueOnce({ ...project, workingName: "A", revision: 1 });
    const { result: committed } = renderHook(() => useProjectRename(project));
    act(() => committed.current.onChangeDraft("A"));
    let committedOutcome!: SubmitOutcome;
    await act(async () => {
      committedOutcome = await committed.current.submit();
    });
    expect(committedOutcome).toEqual({ kind: "committed" });

    // The submitted value commits, but a newer draft was typed meanwhile:
    // the currently displayed draft was never saved, so this must be
    // distinguishable from a plain "committed" outcome.
    let resolveStale!: (value: ProjectSummary) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveStale = resolve;
      }),
    );
    const { result: stale } = renderHook(() => useProjectRename(project));
    act(() => stale.current.onChangeDraft("A"));
    let staleOutcome!: Promise<SubmitOutcome>;
    act(() => {
      staleOutcome = stale.current.submit();
    });
    act(() => stale.current.onChangeDraft("B"));
    act(() => resolveStale({ ...project, workingName: "A", revision: 1 }));
    await act(async () => {
      expect(await staleOutcome).toEqual({ kind: "committed-stale" });
    });

    // The save fails outright: "failed".
    renameProjectMock.mockRejectedValueOnce(new Error("disk full"));
    const { result: failed } = renderHook(() => useProjectRename(project));
    act(() => failed.current.onChangeDraft("A"));
    let failedOutcome!: SubmitOutcome;
    await act(async () => {
      failedOutcome = await failed.current.submit();
    });
    expect(failedOutcome).toEqual({ kind: "failed" });
  });
});
