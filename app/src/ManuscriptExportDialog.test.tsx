import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import {
  chooseManuscriptExportDestination,
  discardManuscriptExport,
  previewManuscriptExport,
  publishManuscriptExport,
  readStory,
} from "./api";
import { ManuscriptExportDialog } from "./ManuscriptExportDialog";
import { exportChapter, exportDestination, exportPreview } from "./manuscriptExportTestFixtures";
import { deferred } from "./chapterTestFixtures";
import type { ManuscriptExportPreview } from "./manuscriptExportTypes";

vi.mock("./api", () => ({
  readStory: vi.fn(),
  previewManuscriptExport: vi.fn(),
  chooseManuscriptExportDestination: vi.fn(),
  publishManuscriptExport: vi.fn(),
  discardManuscriptExport: vi.fn(),
}));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readStory).mockResolvedValue({ globalRevision: 4, chapters: [exportChapter()] });
  vi.mocked(previewManuscriptExport).mockResolvedValue(exportPreview());
  vi.mocked(chooseManuscriptExportDestination).mockResolvedValue(exportDestination());
  vi.mocked(publishManuscriptExport).mockResolvedValue({
    path: "/exports/World.md",
    chapterCount: 1,
    wordCount: 4,
    bytesWritten: 50,
  });
  vi.mocked(discardManuscriptExport).mockResolvedValue(undefined);
});
async function setup(beforePreview = vi.fn().mockResolvedValue(true), onClose = vi.fn()) {
  const rendered = render(
    <ManuscriptExportDialog
      projectId="project"
      beforePreview={beforePreview}
      onOperation={vi.fn()}
      onClose={onClose}
    />,
  );
  await screen.findByRole("button", { name: "Select all active Chapters" });
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Select all active Chapters" })).toBeEnabled(),
  );
  return { ...rendered, beforePreview, onClose };
}
async function preview() {
  fireEvent.click(screen.getByRole("button", { name: "Select all active Chapters" }));
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  await screen.findByLabelText("Exact Markdown preview");
}

it("bounds 120 Chapters, retains off-page selections and excludes inactive Chapters unless explicitly chosen", async () => {
  const chapters = Array.from({ length: 120 }, (_, i) =>
    exportChapter(`chapter-${i}`, { title: `Chapter ${i}`, readingRank: i }),
  );
  vi.mocked(readStory).mockResolvedValue({
    globalRevision: 4,
    chapters: [
      ...chapters,
      exportChapter("archive", { title: "Older", workspaceState: "archived" }),
      exportChapter("trash", { title: "Deleted", workspaceState: "trashed" }),
    ],
  });
  await setup();
  expect(
    within(screen.getByRole("list", { name: "Chapters to export" })).getAllByRole("checkbox"),
  ).toHaveLength(20);
  fireEvent.change(screen.getByRole("searchbox", { name: "Find Chapters" }), {
    target: { value: "Chapter 119" },
  });
  fireEvent.click(screen.getByRole("checkbox", { name: "Chapter 119 4 words" }));
  fireEvent.change(screen.getByRole("searchbox", { name: "Find Chapters" }), {
    target: { value: "" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Select all active Chapters" }));
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  await waitFor(() => expect(previewManuscriptExport).toHaveBeenCalled());
  const selected = vi.mocked(previewManuscriptExport).mock.calls[0][1];
  expect(selected).toHaveLength(120);
  expect(selected).not.toContain("archive");
  expect(selected).not.toContain("trash");
  expect(selected.filter((id) => id === "chapter-119")).toHaveLength(1);
});

it("shows exact inert output, acknowledged scope and format notes before any native file chooser", async () => {
  const text = "# 名称\n\n<script>unsafe()</script>\n<u>Underlined</u>\n";
  vi.mocked(previewManuscriptExport).mockResolvedValue(
    exportPreview({ markdown: text, formatNotes: ["Underline uses generated HTML."] }),
  );
  const { beforePreview } = await setup();
  expect(screen.getByRole("button", { name: "Preview export" })).toBeDisabled();
  await preview();
  expect(beforePreview).toHaveBeenCalledOnce();
  expect(screen.getByLabelText("Exact Markdown preview")).toHaveValue(text);
  expect(document.querySelector("script")).toBeNull();
  expect(screen.getByText("Underline uses generated HTML.")).toBeVisible();
  expect(screen.getByText(/Plan, Notes and Summary stay/)).toBeVisible();
  expect(chooseManuscriptExportDestination).not.toHaveBeenCalled();
  expect(publishManuscriptExport).not.toHaveBeenCalled();
});

it("blocks preview when writing cannot be drained and keeps the selection", async () => {
  await setup(vi.fn().mockResolvedValue(false));
  fireEvent.click(screen.getByRole("button", { name: "Select all active Chapters" }));
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  await screen.findByRole("alert");
  expect(previewManuscriptExport).not.toHaveBeenCalled();
  expect(screen.getByRole("checkbox")).toBeChecked();
});

it("requires explicit inactive selection and displays the acknowledged reading order", async () => {
  vi.mocked(readStory).mockResolvedValue({
    globalRevision: 4,
    chapters: [
      exportChapter("later", { title: "Later", readingRank: 2 }),
      exportChapter("earlier", { title: "Earlier", readingRank: 1, workspaceState: "archived" }),
    ],
  });
  vi.mocked(previewManuscriptExport).mockResolvedValue(
    exportPreview({
      chapters: [
        { id: "earlier", title: "Earlier", workspaceState: "archived", wordCount: 4 },
        { id: "later", title: "Later", workspaceState: "active", wordCount: 4 },
      ],
    }),
  );
  await setup();
  fireEvent.click(screen.getByRole("checkbox", { name: "Later 4 words" }));
  fireEvent.change(screen.getByLabelText("Chapter list"), { target: { value: "archived" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Earlier 4 words · archived" }));
  expect(screen.getByText(/Selection includes 1 archived or trashed Chapters/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  await screen.findByLabelText("Exact Markdown preview");
  expect(previewManuscriptExport).toHaveBeenCalledWith("project", ["later", "earlier"]);
  fireEvent.click(screen.getByText("Included Chapters, in reading order"));
  const order = within(screen.getByRole("region", { name: "Export preview" })).getAllByRole(
    "listitem",
  );
  expect(order.map((item) => item.textContent)).toEqual([
    "Earlier4 words · archived",
    "Later4 words · active",
  ]);
});

it("retries a failed Chapter load and preserves selection after a preview failure", async () => {
  vi.mocked(readStory).mockRejectedValueOnce(new Error("Cannot read Chapters yet."));
  vi.mocked(previewManuscriptExport).mockRejectedValueOnce(
    new Error("Newer writing must be reviewed."),
  );
  render(
    <ManuscriptExportDialog
      projectId="project"
      beforePreview={vi.fn().mockResolvedValue(true)}
      onOperation={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Retry loading Chapters" }));
  fireEvent.click(await screen.findByRole("button", { name: "Select all active Chapters" }));
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Newer writing must be reviewed.");
  expect(screen.getByRole("checkbox")).toBeChecked();
  expect(chooseManuscriptExportDestination).not.toHaveBeenCalled();
});

it("treats native Save As cancellation quietly and retains the reviewed preview", async () => {
  vi.mocked(chooseManuscriptExportDestination).mockResolvedValue(null);
  await setup();
  await preview();
  fireEvent.click(screen.getByRole("button", { name: "Choose export file…" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Choose export file…" })).toBeEnabled(),
  );
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Exact Markdown preview")).toBeVisible();
  expect(publishManuscriptExport).not.toHaveBeenCalled();
});

it("requires an explicit Replace after native destination review and acknowledges success only after publication", async () => {
  vi.mocked(chooseManuscriptExportDestination).mockResolvedValue(
    exportDestination({
      replacesExisting: true,
      existingBytes: 20,
      existingModifiedAt: "2026-01-01T00:00:00Z",
    }),
  );
  const pending = deferred<Awaited<ReturnType<typeof publishManuscriptExport>>>();
  vi.mocked(publishManuscriptExport).mockReturnValueOnce(pending.promise);
  await setup();
  await preview();
  fireEvent.click(screen.getByRole("button", { name: "Choose export file…" }));
  const replace = await screen.findByRole("button", { name: "Replace existing file" });
  expect(screen.getByText("/exports/World.md")).toBeVisible();
  expect(publishManuscriptExport).not.toHaveBeenCalled();
  fireEvent.click(replace);
  expect(publishManuscriptExport).toHaveBeenCalledWith("project", "preview", "destination", true);
  expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
  expect(screen.queryByText(/Exported 1 Chapters/)).not.toBeInTheDocument();
  await act(async () =>
    pending.resolve({ path: "/exports/World.md", chapterCount: 1, wordCount: 4, bytesWritten: 50 }),
  );
  expect(screen.getByText("Exported 1 Chapters · 4 words.")).toBeVisible();
});

it("invalidates both review steps after a stale source or changed file failure, preserving selection", async () => {
  vi.mocked(publishManuscriptExport).mockRejectedValue(new Error("Project changed since preview."));
  await setup();
  await preview();
  fireEvent.click(screen.getByRole("button", { name: "Choose export file…" }));
  fireEvent.click(await screen.findByRole("button", { name: "Export manuscript" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Create a new preview");
  expect(screen.queryByLabelText("Exact Markdown preview")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Export manuscript" })).not.toBeInTheDocument();
  expect(screen.getByRole("checkbox")).toBeChecked();
  expect(discardManuscriptExport).toHaveBeenCalledWith("project", "preview");
});

it("discards old preview tokens when the selection changes or the dialog closes", async () => {
  const { unmount } = await setup();
  await preview();
  fireEvent.click(screen.getByRole("checkbox"));
  await waitFor(() => expect(discardManuscriptExport).toHaveBeenCalledWith("project", "preview"));
  vi.mocked(previewManuscriptExport).mockResolvedValue(
    exportPreview({ previewId: "next-preview" }),
  );
  await preview();
  unmount();
  expect(discardManuscriptExport).toHaveBeenCalledWith("project", "next-preview");
});

it("releases a delayed preview after cancellation and does not open Save As", async () => {
  const pending = deferred<ManuscriptExportPreview>();
  vi.mocked(previewManuscriptExport).mockReturnValueOnce(pending.promise);
  const { onClose, unmount } = await setup();
  fireEvent.click(screen.getByRole("button", { name: "Select all active Chapters" }));
  fireEvent.click(screen.getByRole("button", { name: "Preview export" }));
  await waitFor(() => expect(previewManuscriptExport).toHaveBeenCalledOnce());
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onClose).toHaveBeenCalledOnce();
  unmount();
  await act(async () => pending.resolve(exportPreview()));
  expect(discardManuscriptExport).toHaveBeenCalledWith("project", "preview");
  expect(chooseManuscriptExportDestination).not.toHaveBeenCalled();
});
