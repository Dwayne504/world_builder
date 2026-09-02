import type { SaveState } from "./types";

export type CloseIntent = "project" | "native-window";
export type CloseDecision =
  | "close-project"
  | "close-native-window"
  | "confirm-unsaved-project"
  | "confirm-unsaved-native-window";

export function decideClose(saveState: SaveState, intent: CloseIntent): CloseDecision {
  if (saveState === "saved") {
    return intent === "native-window" ? "close-native-window" : "close-project";
  }
  return intent === "native-window" ? "confirm-unsaved-native-window" : "confirm-unsaved-project";
}
