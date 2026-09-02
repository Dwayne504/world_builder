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
