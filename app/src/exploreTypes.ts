export interface ExploreView {
  query: string;
  categoryId: string | null;
  typeId: string | null;
  capability: string | null;
  workspaceState: "active" | "archived" | "trashed" | "all";
  relationship: {
    definitionId: string;
    perspective: "source" | "target";
    otherEntryId: string | null;
    includeContained: boolean;
  } | null;
  page: number;
  pageSize: number;
}
export const initialExploreView: ExploreView = {
  query: "",
  categoryId: null,
  typeId: null,
  capability: null,
  workspaceState: "active",
  relationship: null,
  page: 0,
  pageSize: 20,
};
export interface ExploreChoice {
  id: string;
  name: string;
  categoryId: string | null;
}
export interface ExploreDefinition {
  id: string;
  name: string;
  forwardLabel: string;
  inverseLabel: string;
  directed: boolean;
  retired: boolean;
}
export interface ExploreEntry {
  id: string;
  name: string;
  category: string;
  typeName: string | null;
  workspaceState: string;
  spatial: boolean;
  relationshipMatch: "current" | "direct" | "contained" | null;
}
export interface ExploreResults {
  globalRevision: number;
  categories: ExploreChoice[];
  types: ExploreChoice[];
  capabilities: string[];
  definitions: ExploreDefinition[];
  selectedOther: ExploreEntry | null;
  issues: string[];
  entries: ExploreEntry[];
  total: number;
  page: number;
  pageSize: number;
}
