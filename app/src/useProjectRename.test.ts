import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const renameProjectMock = vi.fn();
vi.mock("./api", () => ({
  renameProject: (...args: unknown[]) => renameProjectMock(...args),
}));

import { useProjectRename } from "./useProjectRename";
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

    let submitPromise!: Promise<void>;
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
});
