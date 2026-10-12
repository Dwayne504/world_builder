import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { WorkspaceRecordsDialog, WorkspaceRecordsSidebar } from "./WorkspaceRecords";
import { navigationRecord, navigationSnapshot } from "./workspaceNavigationTestFixtures";

it("bounds and searches large lists, opens stable identities and collapses sections", () => {
  const open = vi.fn();
  const records = Array.from({ length: 100 }, (_, i) =>
    navigationRecord(`record-${i}`, { label: `Long record ${i}` }),
  );
  render(
    <WorkspaceRecordsSidebar
      snapshot={navigationSnapshot({ pins: records, recents: records })}
      error={null}
      disabled={false}
      pending={false}
      onOpen={open}
      onRetry={vi.fn()}
      onManage={vi.fn()}
    />,
  );
  const pins = screen.getByRole("list", { name: "Pinned records" });
  expect(within(pins).getAllByRole("button")).toHaveLength(5);
  fireEvent.change(screen.getByRole("searchbox", { name: "Find pinned records" }), {
    target: { value: "Long record 99" },
  });
  fireEvent.click(within(pins).getByRole("button", { name: "Long record 99 Entry" }));
  expect(open).toHaveBeenCalledWith(records[99]);
  fireEvent.click(screen.getByRole("button", { name: /^Pinned 100/ }));
  expect(screen.queryByRole("list", { name: "Pinned records" })).not.toBeInTheDocument();
  expect(screen.getByRole("list", { name: "Recent records" })).toBeVisible();
});

it("shows renamed, inactive and missing records and lets missing pins be removed", () => {
  const apply = vi.fn(),
    open = vi.fn();
  render(
    <WorkspaceRecordsDialog
      open
      onClose={vi.fn()}
      snapshot={navigationSnapshot({
        pins: [
          navigationRecord("stable", {
            label: "New title",
            recordKind: "story_unit",
            workspaceState: "archived",
          }),
          navigationRecord("lost", { label: "Unavailable record", workspaceState: "missing" }),
        ],
      })}
      error={null}
      pending={false}
      disabled={false}
      onOpen={open}
      onApply={apply}
      onRetry={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "New title Chapter · archived" }));
  expect(open).toHaveBeenCalledWith(expect.objectContaining({ recordId: "stable" }));
  expect(screen.getByRole("button", { name: "Unavailable record Entry · missing" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Unpin Unavailable record" }));
  expect(apply).toHaveBeenCalledWith({
    kind: "pin",
    target: { recordKind: "entry", recordId: "lost" },
    pinned: false,
  });
});

it("waits for acknowledged retention changes and clears only recent metadata", () => {
  const apply = vi.fn();
  render(
    <WorkspaceRecordsDialog
      open
      onClose={vi.fn()}
      snapshot={navigationSnapshot({ recents: [navigationRecord("record")] })}
      error={null}
      pending={false}
      disabled={false}
      onOpen={vi.fn()}
      onApply={apply}
      onRetry={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByRole("combobox", { name: "Keep recent records" }), {
    target: { value: "50" },
  });
  expect(apply).toHaveBeenCalledWith({ kind: "set_recent_limit", limit: 50 });
  expect(screen.getByRole("combobox", { name: "Keep recent records" })).toHaveValue("20");
  fireEvent.click(screen.getByRole("button", { name: "Clear recent records" }));
  expect(apply).toHaveBeenLastCalledWith({ kind: "clear_recents" });
});

it("shows a nonmodal failure and leaves existing shortcuts usable while retry is available", () => {
  const retry = vi.fn();
  render(
    <WorkspaceRecordsSidebar
      snapshot={navigationSnapshot({ pins: [navigationRecord("record")] })}
      error="Unavailable"
      disabled={false}
      pending={false}
      onOpen={vi.fn()}
      onRetry={retry}
      onManage={vi.fn()}
    />,
  );
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "record Entry" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "Retry shortcuts" }));
  expect(retry).toHaveBeenCalledOnce();
});
