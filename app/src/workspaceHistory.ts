/** Session navigation stores identity and presentation, never copies of authored data. */
export interface WorkspaceLocation {
  entryId: string | null;
  categoryId: string;
  typeId: string;
  scrollY: number;
  collapsedGroups: string[];
  focusKey: string | null;
}
export interface WorkspaceHistory {
  locations: WorkspaceLocation[];
  index: number;
}
export const initialLocation: WorkspaceLocation = {
  entryId: null,
  categoryId: "",
  typeId: "",
  scrollY: 0,
  collapsedGroups: [],
  focusKey: null,
};
export function visit(history: WorkspaceHistory, location: WorkspaceLocation): WorkspaceHistory {
  const locations = [...history.locations.slice(0, history.index + 1), location].slice(-100);
  return { locations, index: locations.length - 1 };
}
export function capture(
  history: WorkspaceHistory,
  scrollY: number,
  collapsedGroups: string[],
  focusKey: string | null,
): WorkspaceHistory {
  return {
    ...history,
    locations: history.locations.map((location, i) =>
      i === history.index ? { ...location, scrollY, collapsedGroups, focusKey } : location,
    ),
  };
}
export interface NavigationIntent {
  location: WorkspaceLocation;
  index?: number;
}
