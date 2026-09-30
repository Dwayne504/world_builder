import { beforeEach, expect, it } from "vitest";
import { readChapterDraft, storeChapterDraft } from "./chapterRecovery";
import { chapterFixture, textDocument } from "./chapterTestFixtures";
const storageKey = "worldcrafter:chapter-recovery:project:chapter";
beforeEach(() => localStorage.clear());

it.each([
  "broken json",
  JSON.stringify({ version: 42, draft: {} }),
  JSON.stringify({
    version: 1,
    revision: 1,
    savedAt: "today",
    draft: {
      documents: {
        manuscript: {
          type: "doc",
          content: [{ type: "futureNode", content: [{ type: "text", text: "Do not lose me" }] }],
        },
      },
    },
  }),
  JSON.stringify({
    version: 1,
    revision: 1,
    savedAt: "today",
    draft: {
      documents: {
        manuscript: {
          type: "doc",
          content: [{ type: "paragraph", attrs: { futureAttribute: "Do not drop me" } }],
        },
      },
    },
  }),
])("preserves corrupt, newer, or lossy recovery content for copying (%#)", (raw) => {
  localStorage.setItem(storageKey, raw);
  const draft = readChapterDraft("project", chapterFixture());
  expect(draft?.readOnlyReason).toBeTruthy();
  expect(draft?.originalRaw).toBe(raw);
  expect(localStorage.getItem(storageKey)).toBe(raw);
});

it("offers only unsaved areas and preserves supported rich formatting", () => {
  const fixture = chapterFixture();
  const notes = textDocument("Remember this");
  notes.content![0].content![0].marks = [{ type: "bold" }];
  storeChapterDraft("project", "chapter", 3, {
    title: fixture.chapter.title,
    documents: { manuscript: fixture.documents[0].content!, notes },
  });
  const recovered = readChapterDraft("project", fixture);
  expect(recovered?.readOnlyReason).toBeUndefined();
  expect(recovered?.draft).toEqual({ title: undefined, documents: { notes } });
  expect(readChapterDraft("different-project", fixture)).toBeNull();
});
