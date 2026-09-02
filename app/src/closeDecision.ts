import type { SaveState } from "./types";

export type CloseDecision = "close" | "confirm-unsaved";

export function decideClose(saveState: SaveState): CloseDecision {
  return saveState === "saved" ? "close" : "confirm-unsaved";
}
