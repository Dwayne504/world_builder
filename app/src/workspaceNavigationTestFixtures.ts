import type { NavigationRecord, NavigationSnapshot } from "./workspaceNavigationTypes";

export const navigationRecord = (
  id: string,
  changes: Partial<NavigationRecord> = {},
): NavigationRecord => ({
  recordKind: "entry",
  recordId: id,
  label: id,
  workspaceState: "active",
  ...changes,
});
export const navigationSnapshot = (
  changes: Partial<NavigationSnapshot> = {},
): NavigationSnapshot => ({
  pins: [],
  recents: [],
  recentLimit: 20,
  ...changes,
});
