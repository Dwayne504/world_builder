import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { AutomaticBackupPanel } from "./AutomaticBackupPanel";
import { automaticBackupFixture } from "./automaticBackupTestFixtures";
import type { AutomaticBackupStatus } from "./automaticBackupTypes";

const change = vi.fn();
const refresh = vi.fn();
function show(status: AutomaticBackupStatus | null, readError: string | null = null) {
  return render(
    <AutomaticBackupPanel
      status={status}
      readError={readError}
      actionError={null}
      changing={false}
      onChange={change}
      onRefresh={refresh}
    />,
  );
}

it("explains cadence and retention, with timestamps and locations kept separate", () => {
  show(
    automaticBackupFixture({
      lastSuccessAt: "2026-01-02T12:00:00Z",
      lastSuccessPath: "/automatic-backups/snapshot",
    }),
  );
  expect(
    screen.getByRole("checkbox", { name: "Automatic backups for open Projects" }),
  ).toBeChecked();
  expect(screen.getByText(/every 15 minutes/)).toHaveTextContent("newest 20 automatic copies");
  expect(screen.getByText(/every 15 minutes/)).toHaveTextContent(
    "Manual and safety copies are kept separately",
  );
  expect(document.querySelector('time[datetime="2026-01-02T12:00:00Z"]')).toBeInTheDocument();
  expect(screen.getByText("Latest copy: /automatic-backups/snapshot")).not.toBeVisible();
  fireEvent.click(screen.getByText("Automatic backup location"));
  expect(screen.getByText("Latest copy: /automatic-backups/snapshot")).toBeVisible();
});

it("offers an explicit application-wide off setting", () => {
  show(automaticBackupFixture());
  fireEvent.click(screen.getByRole("checkbox"));
  expect(change).toHaveBeenCalledWith(false);
});

it("shows a backup failure without an alert or blocking dialog", () => {
  show(automaticBackupFixture({ error: "Backup folder is unavailable" }));
  expect(screen.getByText("Backup folder is unavailable")).toBeVisible();
  expect(screen.getByText(/Autosave is separate/)).toBeVisible();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Refresh backup status" }));
  expect(refresh).toHaveBeenCalled();
});

it("does not guess a preference when it cannot be read", () => {
  show(
    automaticBackupFixture({ enabled: null, error: "Preferences need recovery", nextDueAt: null }),
  );
  expect(screen.getByRole("checkbox")).toBeDisabled();
  expect(screen.getByText(/unavailable until preferences can be read/)).toBeVisible();
  expect(screen.queryByText("Automatic backups are off.")).not.toBeInTheDocument();
});

it("makes unreadable status visible and disables stale preference controls", () => {
  show(automaticBackupFixture(), "Status could not be read");
  expect(screen.getByText("Status could not be read")).toBeVisible();
  expect(screen.getByRole("checkbox")).toBeDisabled();
});

it("keeps running and disabled states clear", () => {
  const view = show(automaticBackupFixture({ running: true }));
  expect(screen.getByText("Creating an automatic backup…")).toBeVisible();
  view.rerender(
    <AutomaticBackupPanel
      status={automaticBackupFixture({ enabled: false, nextDueAt: null })}
      readError={null}
      actionError={null}
      changing={false}
      onChange={change}
      onRefresh={refresh}
    />,
  );
  expect(screen.getByText("Automatic backups are off.")).toBeVisible();
  expect(screen.queryByText("Next check for changes")).not.toBeInTheDocument();
});
