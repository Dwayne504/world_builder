import type { JSONContent } from "@tiptap/react";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";
import { canEditRecoveredDocument } from "./writingSchema";

export interface EntryDescriptionRecovery {
  version: 1;
  revision: number;
  savedAt: string;
  content: JSONContent | null;
  readOnlyReason?: string;
  originalRaw?: string;
}

const key = (projectId: string, entryId: string) =>
  `worldcrafter:entry-description-recovery:${projectId}:${entryId}`;

/** A best-effort emergency draft outside the Project, never a Saved acknowledgement. */
export function storeEntryDescriptionDraft(
  projectId: string,
  entryId: string,
  revision: number,
  content: JSONContent | null,
): boolean {
  try {
    if (content === null) localStorage.removeItem(key(projectId, entryId));
    else
      localStorage.setItem(
        key(projectId, entryId),
        JSON.stringify({ version: 1, revision, savedAt: new Date().toISOString(), content }),
      );
    return true;
  } catch {
    return false;
  }
}

export function readEntryDescriptionDraft(
  projectId: string,
  snapshot: EntryDescriptionSnapshot,
): EntryDescriptionRecovery | null {
  let raw: string | null = null;
  const unsupported = (): EntryDescriptionRecovery => ({
    version: 1,
    revision: 0,
    savedAt: "",
    content: null,
    readOnlyReason:
      "This local recovery copy is damaged or uses an unsupported format. Its original data is preserved below.",
    originalRaw: raw ?? "",
  });
  try {
    raw = localStorage.getItem(key(projectId, snapshot.entryId));
    if (!raw) return null;
    const value = JSON.parse(raw) as EntryDescriptionRecovery;
    if (
      !value ||
      value.version !== 1 ||
      !Number.isInteger(value.revision) ||
      value.revision < 0 ||
      typeof value.savedAt !== "string" ||
      !canEditRecoveredDocument(value.content)
    )
      return unsupported();
    // A crash after acknowledgement can leave an already-saved emergency copy.
    return JSON.stringify(value.content) === JSON.stringify(snapshot.document?.content)
      ? null
      : value;
  } catch {
    return raw ? unsupported() : null;
  }
}
