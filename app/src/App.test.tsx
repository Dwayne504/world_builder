import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSummary } from "./types";

const project: ProjectSummary = {
  projectId: "0198c000-0000-7000-8000-000000000000",
  workingName: "Tortuga",
  revision: 0,
  packagePath: "/tmp/Tortuga.wcproj",
  formatVersion: 1,
  schemaVersion: 1,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-01T00:00:00Z",
};

const closeProjectMock = vi.fn();
const renameProjectMock = vi.fn();
const openProjectMock = vi.fn();
const nativeWindowCloseMock = vi.fn();
const onCloseRequestedMock = vi.fn();
let closeRequestedHandler: ((event: { preventDefault: () => void }) => void) | undefined;

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    close: (...args: unknown[]) => nativeWindowCloseMock(...args),
    onCloseRequested: (...args: unknown[]) => onCloseRequestedMock(...args),
  }),
}));

vi.mock("./api", () => ({
  AppCommandError: class AppCommandError extends Error {
    kind: string;
    constructor(dto: { kind: string; message: string }) {
      super(dto.message);
      this.kind = dto.kind;
    }
  },
  createProject: vi.fn(),
  openProject: (...args: unknown[]) => openProjectMock(...args),
  restoreBackupAsCopy: vi.fn(),
  createBackup: vi.fn(),
  closeProject: (...args: unknown[]) => closeProjectMock(...args),
  renameProject: (...args: unknown[]) => renameProjectMock(...args),
}));

import App from "./App";
import { AppCommandError } from "./api";

function backendError(kind: string, message: string): AppCommandError {
  return new AppCommandError({ kind, message });
}

async function renderHomeAndFailOpen(kind: string) {
  openProjectMock.mockRejectedValueOnce(backendError(kind, `${kind} diagnostic detail`));
  render(<App />);
  fireEvent.change(screen.getByLabelText("open-project-path"), {
    target: { value: "/tmp/Tortuga.wcproj" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
  await waitFor(() => expect(screen.getAllByRole("alert").length).toBeGreaterThan(0));
}

async function openTheProjectScreen() {
  openProjectMock.mockResolvedValueOnce(project);
  const view = render(<App />);
  fireEvent.change(screen.getByLabelText("open-project-path"), {
    target: { value: "/tmp/Tortuga.wcproj" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
  await waitFor(() => screen.getByTestId("project-id"));
  return view;
}

function enableTauriWindow() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    value: {},
    configurable: true,
  });
}

describe("Project screen Saved contract", () => {
  beforeEach(() => {
    closeProjectMock.mockReset();
    renameProjectMock.mockReset();
    openProjectMock.mockReset();
    nativeWindowCloseMock.mockReset();
    closeRequestedHandler = undefined;
    onCloseRequestedMock.mockReset();
    onCloseRequestedMock.mockImplementation(
      (handler: (event: { preventDefault: () => void }) => void) => {
        closeRequestedHandler = handler;
        return Promise.resolve(vi.fn());
      },
    );
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("shows the Project ID unchanged after a successful rename", async () => {
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });

    await openTheProjectScreen();
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saved"));
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);
  });

  it("refuses to silently close when a rename has failed and is still pending", async () => {
    renameProjectMock.mockRejectedValueOnce(new Error("disk full"));

    await openTheProjectScreen();

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Broken Rename" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(screen.getByTestId("save-state").textContent).toBe("Failed to save"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Close Project" }));

    // Close must be blocked/warned, not silently succeed.
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the Project/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Close Project anyway (discard changes)" }),
    ).toBeInTheDocument();
  });

  it("keeps native close intent when the app close request is confirmed", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);

    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Unsaved Rename" },
    });

    const preventDefault = vi.fn();
    await act(async () => {
      closeRequestedHandler?.({ preventDefault });
    });

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the app/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));

    await waitFor(() =>
      expect(closeProjectMock).toHaveBeenCalledWith("0198c000-0000-7000-8000-000000000000"),
    );
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not downgrade a pending native close when the in-app button is clicked", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));
    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Unsaved Rename" },
    });
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    fireEvent.click(screen.getByRole("button", { name: "Close Project" }));
    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not call the rename API merely because a dirty draft blurred during native close", async () => {
    enableTauriWindow();
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = screen.getByLabelText("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    // Native window closing blurs the focused input before/while handling
    // the close: simulate that here. This must not trigger a save.
    fireEvent.blur(input);
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    expect(renameProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the app/i)).toBeInTheDocument();
  });

  it("confirming discard from a dirty draft closes without ever submitting it", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = screen.getByLabelText("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.blur(input);
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
    expect(renameProjectMock).not.toHaveBeenCalled();
  });

  it("in-app Close Project discards a dirty draft without submitting it", async () => {
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();

    const input = screen.getByLabelText("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.blur(input);

    fireEvent.click(screen.getByRole("button", { name: "Close Project" }));
    fireEvent.click(screen.getByRole("button", { name: "Close Project anyway (discard changes)" }));

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    expect(renameProjectMock).not.toHaveBeenCalled();
  });

  it("waits for an explicit save already in flight instead of pretending to cancel it", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    const preventDefault = vi.fn();
    await act(async () => closeRequestedHandler?.({ preventDefault }));

    // While the save is in flight, close must not offer/claim a discard.
    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "Close app anyway (discard changes)" }),
    ).not.toBeInTheDocument();

    closeProjectMock.mockResolvedValueOnce(undefined);
    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
      await Promise.resolve();
    });

    // A successful in-flight save completes the pending close afterwards.
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("keeps the app open with an honest failed state when the in-flight save fails", async () => {
    enableTauriWindow();
    let rejectRename!: (reason: Error) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectRename = reject;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    await act(async () => {
      rejectRename(new Error("disk full"));
      await Promise.resolve();
    });

    // Must not close, and must not claim the failed save was "discarded".
    await waitFor(() =>
      expect(screen.getByTestId("save-state").textContent).toBe("Failed to save"),
    );
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
  });

  it("completes a successful in-flight save's pending close from the explicit outcome, not a rerender", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    closeProjectMock.mockResolvedValueOnce(undefined);
    // Resolve the rename and flush its `.then` continuation, but do not
    // flush any subsequent scheduler/render pass first: if the close
    // continuation depended on `saveState` having already re-rendered
    // rather than on the explicit resolved outcome, this would still show
    // the race. It must still complete the close correctly.
    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
    });

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not close when a newer draft was typed while an older save was in flight", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = screen.getByLabelText("project-working-name");
    fireEvent.change(input, { target: { value: "Tortuga Prime" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    // A newer draft appears while the older submit is still in flight.
    fireEvent.change(input, { target: { value: "Tortuga Prime Newer" } });

    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
    });

    // The older save committed, but the currently displayed draft never
    // did: closing now would silently discard "Tortuga Prime Newer".
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Pending"));
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
  });

  it("does not re-register the native close listener merely because the draft or save state changes", async () => {
    enableTauriWindow();
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(screen.getByLabelText("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saved"));

    // Typing (dirty), saving, and settling back to saved must not churn the
    // native close listener effect: it should register exactly once.
    expect(onCloseRequestedMock).toHaveBeenCalledTimes(1);
  });

  it("cleans up a native close listener even when registration resolves after unmount", async () => {
    enableTauriWindow();
    let resolveListener!: (listener: () => void) => void;
    onCloseRequestedMock.mockImplementationOnce(
      (handler: (event: { preventDefault: () => void }) => void) => {
        closeRequestedHandler = handler;
        return new Promise<() => void>((resolve) => {
          resolveListener = resolve;
        });
      },
    );

    const view = await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    view.unmount();

    const lateUnlisten = vi.fn();
    resolveListener(lateUnlisten);

    await waitFor(() => expect(lateUnlisten).toHaveBeenCalledTimes(1));
  });
});

describe("Home screen stale-lock recovery", () => {
  beforeEach(() => {
    openProjectMock.mockReset();
  });

  it("offers explicit recovery only when the backend reports lock_recovery_required", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");

    // The entered path is retained after the failed open.
    expect(screen.getByLabelText("open-project-path")).toHaveValue("/tmp/Tortuga.wcproj");

    // Understandable wording leads; jargon stays out of the primary text.
    expect(screen.getByText(/not closed properly/i)).toBeInTheDocument();
    expect(screen.getByText(/lock_recovery_required diagnostic detail/)).toBeInTheDocument();

    const recover = screen.getByRole("button", { name: "Recover lock and open Project" });
    expect(recover).toBeInTheDocument();
    // The safety caveat is shown next to the action.
    expect(screen.getByText(/no other Worldcrafter instance/i)).toBeInTheDocument();

    // The recovery action opens the same path with forceStaleLockRecovery=true.
    openProjectMock.mockResolvedValueOnce(project);
    fireEvent.click(recover);
    await waitFor(() => screen.getByTestId("project-id"));
    expect(openProjectMock).toHaveBeenLastCalledWith("/tmp/Tortuga.wcproj", true);
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);
  });

  it("invalidates recovery immediately when the Package path is edited", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");
    const path = screen.getByLabelText("open-project-path");

    fireEvent.change(path, { target: { value: "/tmp/Other.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();

    fireEvent.change(path, { target: { value: "/tmp/Tortuga.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
  });

  it("never authorizes recovery for an old path whose request finishes after an edit", async () => {
    let rejectOpen: ((reason: AppCommandError) => void) | undefined;
    openProjectMock.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectOpen = reject;
      }),
    );
    render(<App />);
    const path = screen.getByLabelText("open-project-path");
    fireEvent.change(path, { target: { value: "/tmp/Old.wcproj" } });
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));

    fireEvent.change(path, { target: { value: "/tmp/New.wcproj" } });
    rejectOpen?.(backendError("lock_recovery_required", "old path diagnostic"));
    await waitFor(() => expect(screen.getByText(/not closed properly/i)).toBeInTheDocument());

    fireEvent.change(path, { target: { value: "/tmp/Old.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    expect(openProjectMock).toHaveBeenCalledWith("/tmp/Old.wcproj");
  });

  it.each([
    ["lock_held", /currently open in another Worldcrafter instance/i],
    ["lock_not_stale", /may still be in use/i],
    ["lock_metadata_corrupt", /lock information is unreadable/i],
  ])("offers no unsafe recovery action for %s", async (kind, wording) => {
    await renderHomeAndFailOpen(kind);

    expect(screen.getByText(wording)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    // The path is retained so the user can retry or edit it.
    expect(screen.getByLabelText("open-project-path")).toHaveValue("/tmp/Tortuga.wcproj");
  });

  it.each([
    ["lock_held", "held by pid 42"],
    ["lock_not_stale", "heartbeat is too recent"],
    ["lock_metadata_corrupt", "invalid lock metadata"],
    ["invalid_package", "unrelated open failure"],
  ])("clears recovery when the latest recovery attempt fails with %s", async (kind, diagnostic) => {
    openProjectMock.mockRejectedValueOnce(
      backendError("lock_recovery_required", "stale lock diagnostic"),
    );
    render(<App />);
    fireEvent.change(screen.getByLabelText("open-project-path"), {
      target: { value: "/tmp/Tortuga.wcproj" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
    await waitFor(() => screen.getByRole("button", { name: "Recover lock and open Project" }));

    openProjectMock.mockRejectedValueOnce(backendError(kind, diagnostic));
    fireEvent.click(screen.getByRole("button", { name: "Recover lock and open Project" }));

    await waitFor(() => expect(screen.getByText(new RegExp(diagnostic))).toBeInTheDocument());
    expect(openProjectMock).toHaveBeenLastCalledWith("/tmp/Tortuga.wcproj", true);
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId("project-id")).not.toBeInTheDocument();
  });

  it("retains recovery only when the latest failure still requires it for the unchanged path", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");
    openProjectMock.mockRejectedValueOnce(
      backendError("lock_recovery_required", "still stale diagnostic"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Recover lock and open Project" }));

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Recover lock and open Project" }),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText(/still stale diagnostic/)).toBeInTheDocument();
  });

  it("does not offer recovery for non-lock open failures", async () => {
    await renderHomeAndFailOpen("invalid_package");
    expect(screen.getByText(/invalid_package/)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
  });
});
