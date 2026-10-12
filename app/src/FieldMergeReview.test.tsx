import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { FieldMergeReview } from "./FieldMergeReview";
import { getPreferences, mergeFields, previewFieldMerge } from "./api";
import type { FieldDefinition, FieldMergePreview } from "./types";

vi.mock("./api", () => ({
  getPreferences: vi.fn(),
  mergeFields: vi.fn(),
  previewFieldMerge: vi.fn(),
  pickDirectory: vi.fn(),
}));
const local: FieldDefinition = {
  id: "local",
  name: "Age",
  kind: "number",
  unit: "years",
  revision: 1,
  retired: false,
  options: [],
  bindings: [{ provider: { kind: "entry", id: "entry" }, label: "Leopold" }],
};
const shared: FieldDefinition = {
  ...local,
  id: "shared",
  bindings: [{ provider: { kind: "type", id: "human" }, label: "Human" }],
};
const preview: FieldMergePreview = {
  globalRevision: 5,
  source: local,
  target: shared,
  blockers: [],
  entries: [
    {
      entryId: "entry",
      name: "Leopold",
      sourceValue: { kind: "number", value: 48 },
      targetValue: null,
      conflict: false,
    },
  ],
};
const commit = vi.fn((action: () => Promise<unknown>) => {
  void action();
});
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(getPreferences).mockResolvedValue({
    defaultProjectsDir: null,
    defaultProjectsDirExists: false,
    defaultBackupsDir: "/Backups",
    defaultBackupsDirExists: true,
    automaticBackupsEnabled: true,
  });
  vi.mocked(previewFieldMerge).mockResolvedValue(preview);
  vi.mocked(mergeFields).mockResolvedValue({
    globalRevision: 6,
    backupPath: "/Backups/copy.wcbackup",
  });
});
async function review() {
  render(
    <FieldMergeReview
      projectId="project"
      definitions={[local, shared]}
      disabled={false}
      onCommit={commit}
    />,
  );
  fireEvent.change(screen.getByLabelText("Keep Field"), { target: { value: "shared" } });
  fireEvent.change(screen.getByLabelText("Duplicate Field"), { target: { value: "local" } });
  fireEvent.click(screen.getByRole("button", { name: "Review merge" }));
  await screen.findByText("Review: Age");
}
it("requires the reviewed values, a destination, and an explicit confirmation before merge", async () => {
  await review();
  expect(screen.getByText("Leopold", { selector: "th" })).toBeVisible();
  expect(screen.getByLabelText("Merge backup folder")).toHaveValue("/Backups");
  const merge = screen.getByRole("button", { name: "Back up and merge" });
  expect(merge).toBeDisabled();
  expect(mergeFields).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.click(merge);
  await waitFor(() =>
    expect(mergeFields).toHaveBeenCalledWith("project", "local", "shared", 5, "/Backups"),
  );
});
it("shows conflict details and disables committing an incompatible merge", async () => {
  vi.mocked(previewFieldMerge).mockResolvedValue({
    ...preview,
    blockers: ["Different values must be resolved first."],
    entries: [
      { ...preview.entries[0], targetValue: { kind: "number", value: 49 }, conflict: true },
    ],
  });
  await review();
  expect(screen.getByRole("alert")).toHaveTextContent("Different values");
  expect(screen.getByText("48")).toBeVisible();
  expect(screen.getByText("49")).toBeVisible();
  expect(screen.getByRole("checkbox")).toBeDisabled();
  expect(screen.getByRole("button", { name: "Back up and merge" })).toBeDisabled();
});
it("invalidates the preview and confirmation when either selected definition changes", async () => {
  await review();
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.change(screen.getByLabelText("Keep Field"), { target: { value: "local" } });
  expect(screen.queryByText("Review: Age")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Back up and merge" })).not.toBeInTheDocument();
  expect(mergeFields).not.toHaveBeenCalled();
});
