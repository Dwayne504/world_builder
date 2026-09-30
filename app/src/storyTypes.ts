import type { JSONContent } from "@tiptap/react";
export type DocumentArea = "manuscript" | "plan" | "notes";
export type WorkspaceState = "active" | "archived" | "trashed";
export interface DocumentEdit {
  area: DocumentArea;
  schemaVersion: number;
  content: JSONContent;
}
export interface RichDocument {
  id: string;
  area: DocumentArea;
  schemaVersion: number;
  content: JSONContent | null;
  plainText: string;
  wordCount: number;
  revision: number;
  readOnlyReason: string | null;
  originalJson: string | null;
}
export interface ChapterSummary {
  id: string;
  title: string;
  workspaceState: WorkspaceState;
  readingRank: number;
  revision: number;
  wordCount: number;
}
export interface StoryRole {
  id: string;
  name: string;
}
export interface StoryLink {
  id: string;
  entryId: string | null;
  label: string;
  workspaceState: string;
  roles: StoryRole[];
}
export interface ChapterSnapshot {
  globalRevision: number;
  chapter: ChapterSummary;
  documents: RichDocument[];
  links: StoryLink[];
  roles: StoryRole[];
}
export interface StoryIndex {
  globalRevision: number;
  chapters: ChapterSummary[];
}
export interface StoryUsage {
  chapter: ChapterSummary;
  roles: StoryRole[];
}
export type StoryCommand =
  | { kind: "create"; title: string }
  | { kind: "save"; chapterId: string; title: string | null; documents: DocumentEdit[] }
  | { kind: "move"; chapterId: string; beforeId: string | null }
  | { kind: "set_state"; chapterId: string; state: WorkspaceState }
  | { kind: "set_link"; chapterId: string; entryId: string; roleIds: string[] }
  | { kind: "unlink"; chapterId: string; linkId: string }
  | { kind: "create_role"; chapterId: string; name: string };
export const chapterLabel = (chapter: ChapterSummary) =>
  chapter.title.trim() || "[Untitled Chapter]";
export type ChapterDraft = {
  title?: string;
  documents: Partial<Record<DocumentArea, JSONContent>>;
};
export interface ChapterController {
  state: import("./types").SaveState;
  submit: () => Promise<import("./useProjectRename").SubmitOutcome>;
  canSubmit: boolean;
  autoFlush?: boolean;
}
