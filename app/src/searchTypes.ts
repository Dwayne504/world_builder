import type { DocumentArea } from "./storyTypes";

export interface SearchView {
  query: string;
  includeInactive: boolean;
  limitPerGroup: number;
}
export type SearchTarget =
  | { kind: "entry"; entryId: string }
  | { kind: "chapter"; chapterId: string; area: DocumentArea }
  | { kind: "relationship"; relationshipId: string };
export interface SearchHit {
  key: string;
  title: string;
  context: string;
  workspaceState: string;
  reason: string;
  excerpt: string;
  target: SearchTarget;
}
export interface SearchResults {
  globalRevision: number;
  groups: {
    kind: "entries" | "chapters" | "structured" | "text";
    total: number;
    hits: SearchHit[];
  }[];
}
export interface EntryAliases {
  globalRevision: number;
  aliases: { id: string; text: string }[];
}
export type AliasCommand = { kind: "add"; text: string } | { kind: "delete"; aliasId: string };
