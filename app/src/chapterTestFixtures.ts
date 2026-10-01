import type { ChapterSnapshot, DocumentArea } from "./storyTypes";
import type { JSONContent } from "@tiptap/react";
export const textDocument = (text: string): JSONContent => ({
  type: "doc",
  content: [{ type: "paragraph", ...(text ? { content: [{ type: "text", text }] } : {}) }],
});
export function chapterFixture(): ChapterSnapshot {
  return {
    globalRevision: 3,
    chapter: {
      id: "chapter",
      title: "The First Step",
      readingRank: 1,
      workspaceState: "active",
      revision: 1,
      wordCount: 2,
    },
    documents: (["manuscript", "plan", "notes"] as DocumentArea[]).map((area) => ({
      id: `doc-${area}`,
      area,
      schemaVersion: 1,
      content: textDocument(area === "manuscript" ? "Thron arrived." : ""),
      plainText: area === "manuscript" ? "Thron arrived.\n" : "",
      wordCount: area === "manuscript" ? 2 : 0,
      revision: 1,
      readOnlyReason: null,
      originalJson: null,
    })),
    links: [],
    roles: [
      { id: "pov", name: "POV" },
      { id: "setting", name: "Setting" },
    ],
  };
}
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
