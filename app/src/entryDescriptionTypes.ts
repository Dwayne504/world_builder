import type { RichDocument, WorkspaceState } from "./storyTypes";

export type EntryDescriptionDocument = Omit<RichDocument, "area">;

export interface EntryDescriptionSnapshot {
  globalRevision: number;
  entryId: string;
  workspaceState: WorkspaceState;
  document: EntryDescriptionDocument | null;
}
