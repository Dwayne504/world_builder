import { beforeEach, expect, it } from "vitest";
import { textDocument } from "./chapterTestFixtures";
import { descriptionFixture } from "./entryDescriptionTestFixtures";
import { readEntryDescriptionDraft, storeEntryDescriptionDraft } from "./entryDescriptionRecovery";

beforeEach(() => localStorage.clear());

it("scopes emergency drafts to Project and Entry, and ignores already acknowledged content", () => {
  storeEntryDescriptionDraft("project", "entry", 3, textDocument("Lore"));
  expect(readEntryDescriptionDraft("other-project", descriptionFixture())).toBeNull();
  expect(
    readEntryDescriptionDraft("project", { ...descriptionFixture(), entryId: "other-entry" }),
  ).toBeNull();
  expect(readEntryDescriptionDraft("project", descriptionFixture())?.content).toEqual(
    textDocument("Lore"),
  );
  expect(readEntryDescriptionDraft("project", descriptionFixture("Lore"))).toBeNull();
});

it.each([
  "damaged original",
  JSON.stringify({ version: 2, revision: 3, savedAt: "today", content: textDocument("Future") }),
  JSON.stringify({
    version: 1,
    revision: 3,
    savedAt: "today",
    content: { type: "doc", content: [{ type: "futureNode" }] },
  }),
])("preserves unsupported recovery data read-only: %s", (raw) => {
  localStorage.setItem("worldcrafter:entry-description-recovery:project:entry", raw);
  expect(readEntryDescriptionDraft("project", descriptionFixture())).toMatchObject({
    content: null,
    originalRaw: raw,
    readOnlyReason: expect.any(String),
  });
  expect(localStorage.getItem("worldcrafter:entry-description-recovery:project:entry")).toBe(raw);
});
