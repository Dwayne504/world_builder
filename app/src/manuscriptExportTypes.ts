export interface ManuscriptExportPreview {
  previewId: string;
  projectId: string;
  globalRevision: number;
  chapters: { id: string; title: string; workspaceState: string; wordCount: number }[];
  wordCount: number;
  markdown: string;
  suggestedFileName: string;
  formatNotes: string[];
}
export interface ManuscriptExportDestination {
  destinationId: string;
  path: string;
  replacesExisting: boolean;
  existingBytes: number | null;
  existingModifiedAt: string | null;
}
export interface ManuscriptExportReceipt {
  path: string;
  chapterCount: number;
  wordCount: number;
  bytesWritten: number;
}
