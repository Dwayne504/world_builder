import type { SaveState } from "./types";

export type CloseIntent = "project" | "native-window";
export type CloseDecision =
  | "close-project"
  | "close-native-window"
  | "confirm-unsaved-project"
  | "confirm-unsaved-native-window"
  | "wait-for-save-project"
  | "wait-for-save-native-window";

/**
 * A save that is already in flight cannot be safely cancelled, so it must
 * never be offered as "discardable" -- doing so would let the user believe
 * Discard cancelled a write that actually keeps running and commits anyway.
 * Instead, close must wait for that save to settle before deciding whether
 * it can proceed (see `App.tsx`'s `waitForSaveThenClose`).
 */
export function decideClose(saveState: SaveState, intent: CloseIntent): CloseDecision {
  if (saveState === "saved") {
    return intent === "native-window" ? "close-native-window" : "close-project";
  }
  if (saveState === "saving") {
    return intent === "native-window" ? "wait-for-save-native-window" : "wait-for-save-project";
  }
  return intent === "native-window" ? "confirm-unsaved-native-window" : "confirm-unsaved-project";
}
