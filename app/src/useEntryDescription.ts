import { readEntryDescription, saveEntryDescription } from "./api";
import { useEntryDocument } from "./useEntryDocument";
export function useEntryDescription(
  projectId: string,
  entryId: string,
  onRevision: (revision: number) => void,
  getRevision: () => number,
) {
  return useEntryDocument(projectId, entryId, onRevision, getRevision, {
    read: () => readEntryDescription(projectId, entryId),
    save: (expected, revision, version, content) =>
      saveEntryDescription(projectId, entryId, expected, revision, version, content),
  });
}
