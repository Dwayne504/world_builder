import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";
import { textDocument } from "./chapterTestFixtures";

export function descriptionFixture(text?: string): EntryDescriptionSnapshot {
  return {
    globalRevision: 3,
    entryId: "entry",
    workspaceState: "active",
    document:
      text === undefined
        ? null
        : {
            id: "description",
            schemaVersion: 1,
            content: textDocument(text),
            plainText: `${text}\n`,
            wordCount: text ? text.trim().split(/\s+/).length : 0,
            revision: 1,
            readOnlyReason: null,
            originalJson: null,
          },
  };
}
