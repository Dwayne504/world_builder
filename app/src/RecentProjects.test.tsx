import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { RecentProjects } from "./RecentProjects";
import {
  AppCommandError,
  forgetRecentProject,
  listRecentProjects,
  openRecentProject,
  pickDirectory,
} from "./api";
import type { RecentProject } from "./types";

vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
  listRecentProjects: vi.fn(),
  openRecentProject: vi.fn(),
  forgetRecentProject: vi.fn(),
  pickDirectory: vi.fn(),
}));
const recent: RecentProject = {
  projectId: "project",
  workingName: "A world",
  packagePath: "/test/A.wcproj",
  lastAccessedAt: "2026-01-01",
  available: true,
};
const opened = vi.fn();
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listRecentProjects).mockResolvedValue([recent]);
});
async function mount() {
  render(<RecentProjects busy={false} onBusy={vi.fn()} onOpened={opened} />);
  await screen.findByRole("button", { name: "A world" });
}
it("opens a named shortcut by identity without displaying the path as its label", async () => {
  await mount();
  expect(screen.getByRole("button", { name: "A world" })).not.toHaveTextContent("/test");
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "A world" })));
  expect(openRecentProject).toHaveBeenCalledWith("project", undefined, false);
  expect(opened).toHaveBeenCalledOnce();
});
it("retains unavailable Projects and permits identity-checked relocation", async () => {
  vi.mocked(listRecentProjects).mockResolvedValue([{ ...recent, available: false }]);
  vi.mocked(pickDirectory).mockResolvedValue("/test/moved.wcproj");
  vi.mocked(openRecentProject).mockRejectedValue(
    new AppCommandError({ kind: "identity_mismatch", message: "wrong project" }),
  );
  await mount();
  expect(screen.getByText("Folder unavailable")).toBeVisible();
  fireEvent.click(screen.getByText("Options"));
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Locate folder…" })));
  expect(openRecentProject).toHaveBeenCalledWith("project", "/test/moved.wcproj", false);
  expect(screen.getByRole("alert")).toHaveTextContent("different Project");
  expect(opened).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "A world" })).toBeVisible();
});
it("requires explicit recovery and keeps the selected Project identity", async () => {
  vi.mocked(openRecentProject)
    .mockRejectedValueOnce(
      new AppCommandError({ kind: "lock_recovery_required", message: "stale" }),
    )
    .mockResolvedValueOnce({} as never);
  await mount();
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "A world" })));
  expect(opened).not.toHaveBeenCalled();
  await act(async () =>
    fireEvent.click(screen.getByRole("button", { name: "Recover lock and open Project" })),
  );
  expect(openRecentProject).toHaveBeenLastCalledWith("project", undefined, true);
  expect(opened).toHaveBeenCalledOnce();
});
it("only removes a shortcut after the backend acknowledges it", async () => {
  vi.mocked(forgetRecentProject)
    .mockRejectedValueOnce(new Error("list is read-only"))
    .mockResolvedValueOnce();
  await mount();
  fireEvent.click(screen.getByText("Options"));
  await act(async () =>
    fireEvent.click(screen.getByRole("button", { name: "Remove from recent list" })),
  );
  expect(screen.getByRole("button", { name: "A world" })).toBeVisible();
  await act(async () =>
    fireEvent.click(screen.getByRole("button", { name: "Remove from recent list" })),
  );
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "A world" })).not.toBeInTheDocument(),
  );
});
