import type { ManuscriptExportDestination, ManuscriptExportPreview } from "./manuscriptExportTypes";
import type { ChapterSummary } from "./storyTypes";
export const exportChapter = (
  id = "chapter",
  changes: Partial<ChapterSummary> = {},
): ChapterSummary => ({
  id,
  title: "First Chapter",
  workspaceState: "active",
  readingRank: 0,
  revision: 1,
  wordCount: 4,
  ...changes,
});
export const exportPreview = (
  changes: Partial<ManuscriptExportPreview> = {},
): ManuscriptExportPreview => ({
  previewId: "preview",
  projectId: "project",
  globalRevision: 4,
  chapters: [{ id: "chapter", title: "First Chapter", workspaceState: "active", wordCount: 4 }],
  wordCount: 4,
  markdown: "# First Chapter\n\nThe moon shines bright.\n",
  suggestedFileName: "World.md",
  formatNotes: [],
  ...changes,
});
export const exportDestination = (
  changes: Partial<ManuscriptExportDestination> = {},
): ManuscriptExportDestination => ({
  destinationId: "destination",
  path: "/exports/World.md",
  replacesExisting: false,
  existingBytes: null,
  existingModifiedAt: null,
  ...changes,
});
