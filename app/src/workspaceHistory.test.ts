import { expect, it } from "vitest";
import { capture, initialLocation, visit } from "./workspaceHistory";

it("preserves filters, scroll and focus while a new branch discards only Forward history", () => {
  const first = { ...initialLocation, categoryId: "characters", typeId: "human" };
  let history = capture({ locations: [first], index: 0 }, 800, ["places"], "leopold");
  history = visit(history, { ...first, entryId: "leopold" });
  history = visit(history, { ...first, entryId: "blade" });
  history = visit({ ...history, index: 1 }, { ...first, entryId: "planet" });
  expect(history.locations.map((location) => location.entryId)).toEqual([
    null,
    "leopold",
    "planet",
  ]);
  expect(history.locations[0]).toMatchObject({
    typeId: "human",
    scrollY: 800,
    focusKey: "leopold",
    collapsedGroups: ["places"],
  });
});
it("bounds session history without losing the active location", () => {
  let history = { locations: [initialLocation], index: 0 };
  for (let i = 0; i < 110; i++)
    history = visit(history, { ...initialLocation, entryId: String(i) });
  expect(history.locations).toHaveLength(100);
  expect(history.locations[history.index].entryId).toBe("109");
});
