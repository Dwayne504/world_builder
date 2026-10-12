export interface NavigationTarget {
  recordKind: "entry" | "story_unit" | "temporal_occurrence";
  recordId: string;
}
export interface NavigationRecord extends NavigationTarget {
  label: string;
  workspaceState: string;
}
export interface NavigationSnapshot {
  pins: NavigationRecord[];
  recents: NavigationRecord[];
  recentLimit: number;
}
export type NavigationCommand =
  | { kind: "visit"; target: NavigationTarget }
  | { kind: "pin"; target: NavigationTarget; pinned: boolean }
  | { kind: "set_recent_limit"; limit: number }
  | { kind: "clear_recents" };

export const navigationKey = (target: NavigationTarget) =>
  `${target.recordKind}:${target.recordId}`;
export const navigationKindLabel = (target: NavigationTarget) =>
  target.recordKind === "entry"
    ? "Entry"
    : target.recordKind === "story_unit"
      ? "Chapter"
      : "Occurrence";
