import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { TypeRetirementReview } from "./TypeRetirementReview";
import { previewTypeRetirement } from "./api";
import { typeRetirementPreview } from "./typeRetirementTestFixtures";
import { deferred } from "./chapterTestFixtures";
import type { TypeRetirementPreview } from "./types";
vi.mock("./api", () => ({ previewTypeRetirement: vi.fn() }));
beforeEach(() => {
  vi.mocked(previewTypeRetirement).mockReset().mockResolvedValue(typeRetirementPreview());
});
const props = {
  projectId: "project",
  typeId: "sword",
  busy: false,
  onConfirm: vi.fn().mockResolvedValue(true),
  onCancel: vi.fn(),
};

it("keeps a thousand affected Entries bounded and searchable with inherited and inactive context", async () => {
  vi.mocked(previewTypeRetirement).mockResolvedValue(
    typeRetirementPreview({
      directEntryCount: 999,
      inheritedEntryCount: 1,
      assignedEntries: Array.from({ length: 1000 }, (_, i) => ({
        id: `entry-${i}`,
        label: `Blade ${i}`,
        typeId: i === 999 ? "cutlass" : "sword",
        typeName: i === 999 ? "Cutlass" : "Sword",
        workspaceState: i === 999 ? "trashed" : "active",
      })),
    }),
  );
  render(<TypeRetirementReview {...props} />);
  await screen.findByRole("button", { name: "Retire Type" });
  expect(screen.getByRole("list")).not.toBeVisible();
  fireEvent.click(screen.getByText("Review existing uses"));
  expect(
    within(screen.getByRole("list", { name: "Existing Type uses" })).getAllByRole("listitem"),
  ).toHaveLength(10);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Blade 999" } });
  expect(screen.getByText("Blade 999")).toBeVisible();
  expect(screen.getByText(/inherits through child Type · trashed/)).toBeVisible();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  fireEvent.change(screen.getByLabelText("Show"), { target: { value: "types" } });
  expect(screen.getByText("Cutlass")).toBeVisible();
  expect(screen.getByText(/Active child Type · remains available/)).toBeVisible();
  fireEvent.change(screen.getByLabelText("Show"), { target: { value: "fields" } });
  expect(screen.getByText("Mass")).toBeVisible();
});

it("requires a fresh review after refusal and submits restoration with its reviewed revision", async () => {
  const confirm = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
  render(<TypeRetirementReview {...props} onConfirm={confirm} />);
  fireEvent.click(await screen.findByRole("button", { name: "Retire Type" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Refresh the review");
  expect(screen.queryByRole("button", { name: "Retire Type" })).not.toBeInTheDocument();
  expect(confirm).toHaveBeenCalledWith(4, true);
  vi.mocked(previewTypeRetirement).mockResolvedValue(
    typeRetirementPreview({ globalRevision: 8, retiredAt: "2026-10-01T12:00:00Z" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Refresh review" }));
  const restore = await screen.findByRole("button", { name: "Restore Type" });
  await act(async () => fireEvent.click(restore));
  expect(confirm).toHaveBeenLastCalledWith(8, false);
});

it("never writes on cancellation or failed preview and ignores a late review for another Type", async () => {
  const pending = deferred<TypeRetirementPreview>();
  vi.mocked(previewTypeRetirement).mockReturnValueOnce(pending.promise);
  const confirm = vi.fn();
  const cancel = vi.fn();
  const rendered = render(
    <TypeRetirementReview {...props} onConfirm={confirm} onCancel={cancel} />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(cancel).toHaveBeenCalledOnce();
  vi.mocked(previewTypeRetirement).mockRejectedValueOnce(new Error("Review unavailable"));
  rendered.rerender(
    <TypeRetirementReview {...props} typeId="another" onConfirm={confirm} onCancel={cancel} />,
  );
  await screen.findByText("Review unavailable");
  await act(async () => pending.resolve(typeRetirementPreview()));
  expect(screen.queryByRole("button", { name: "Retire Type" })).not.toBeInTheDocument();
  expect(confirm).not.toHaveBeenCalled();
});
