import type { DocumentArea } from "./storyTypes";

export interface SearchRequest {
  query: string;
  includeInactive: boolean;
  limitPerGroup: number;
  entryId?: string;
  structuredKind?: "fields" | "relationships" | "chapters";
  textArea?: DocumentArea;
  storyRoleId?: string;
}
export interface SearchView extends SearchRequest {
  entryName?: string;
  storyRoleName?: string;
  expandedHits?: string[];
  extendedHits?: string[];
}
export type SearchTarget =
  | { kind: "occurrence"; occurrenceId: string }
  | { kind: "story_role"; roleId: string; name: string }
  | { kind: "entry"; entryId: string }
  | { kind: "chapter"; chapterId: string; area: DocumentArea }
  | { kind: "relationship"; relationshipId: string; perspectiveEntryId?: string };
export interface SearchHit {
  key: string;
  title: string;
  context: string;
  workspaceState: string;
  reason: string;
  excerpt: string;
  preview?: string;
  target: SearchTarget;
}
export interface SearchResults {
  globalRevision: number;
  groups: {
    kind: "entries" | "chapters" | "roles" | "structured" | "text" | "timeline";
    total: number;
    hits: SearchHit[];
  }[];
}
export interface EntryAliases {
  globalRevision: number;
  aliases: { id: string; text: string }[];
}
export type AliasCommand = { kind: "add"; text: string } | { kind: "delete"; aliasId: string };
