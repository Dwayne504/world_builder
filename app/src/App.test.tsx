import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  render(<App />);
  fireEvent.change(screen.getByLabelText("open-project-path"), {
    target: { value: "/tmp/Tortuga.wcproj" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
  await waitFor(() => screen.getByTestId("project-id"));
}

describe("Project screen Saved contract", () => {
  beforeEach(() => {
    closeProjectMock.mockReset();
    renameProjectMock.mockReset();
    openProjectMock.mockReset();
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
    expect(screen.getByText(/unsaved changes/i)).toBeInTheDocument();
  });
});
