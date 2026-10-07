/**
 * Wire types shared with the Tauri boundary (see
 * `src-tauri/src/tauri_boundary/dto.rs`). Kept intentionally small: this is
 * the Milestone 01 Trust Foundation slice, not the full domain model.
 */

export interface RecentProject {
  projectId: string;
  workingName: string;
  packagePath: string;
  lastAccessedAt: string;
  available: boolean;
}

export interface ProjectSummary {
  recentProjectsWarning?: string | null;
  projectId: string;
  workingName: string;
  revision: number;
  packagePath: string;
  formatVersion: number;
  schemaVersion: number;
  createdAt: string;
  updatedAt: string;
}

export interface AppErrorDto {
  kind: string;
  message: string;
}

/** Distinct Saved-state values shown to the user (see README "Saved contract"). */
export type SaveState = "saved" | "dirty" | "saving" | "failed";

export interface Category {
  id: string;
  name: string;
  isUncategorized: boolean;
  revision: number;
  globalRevision: number;
}

export interface TypeDef {
  id: string;
  categoryId: string;
  parentTypeId: string | null;
  name: string;
  revision: number;
  globalRevision: number;
}

export interface Entry {
  workspaceState: "active" | "archived" | "trashed";
  id: string;
  categoryId: string;
  typeId: string | null;
  authoredName: string | null;
  displayName: string;
  revision: number;
  globalRevision: number;
}

export interface Preferences {
  defaultProjectsDir: string | null;
  defaultProjectsDirExists: boolean;
  defaultBackupsDir: string | null;
  defaultBackupsDirExists: boolean;
}

export type FieldKind =
  "short_text" | "rich_text" | "number" | "boolean" | "choice" | "multi_choice" | "relationship";
export interface FieldProjection {
  relationshipDefinitionId: string;
  perspective: "source" | "target";
}
export type OtherEntry =
  | { kind: "existing"; id: string }
  | { kind: "create"; name: string | null; categoryId: string | null };
export interface FieldProvider {
  kind: "category" | "type" | "entry";
  id: string;
}
export interface ChoiceOption {
  id: string;
  label: string;
  retired: boolean;
}
export interface FieldDefinition {
  id: string;
  name: string;
  kind: FieldKind;
  unit?: string | null;
  projection?: FieldProjection | null;
  retired: boolean;
  revision: number;
  options: ChoiceOption[];
  bindings: { provider: FieldProvider; label: string }[];
}
export interface RichFieldValue {
  schemaVersion: number;
  content: import("@tiptap/react").JSONContent | null;
  revision: number;
  plainText: string;
  readOnlyReason: string | null;
  originalJson: string | null;
}
export type FieldValue =
  | { kind: "rich_text"; value: RichFieldValue }
  | { kind: "text"; value: string }
  | { kind: "number"; value: number }
  | { kind: "boolean"; value: boolean }
  | { kind: "choices"; value: string[] };
export interface EntryField {
  definition: FieldDefinition;
  available: boolean;
  hidden?: boolean;
  defaultSources?: FieldDefinition["bindings"];
  value: FieldValue | null;
  projectedRelationships?: Relationship[];
}
export interface EntryFields {
  globalRevision: number;
  fields: EntryField[];
  definitions: FieldDefinition[];
}
export type FieldCommand =
  | {
      kind: "create_projection";
      name: string;
      relationshipDefinitionId: string;
      perspective: "source" | "target";
      provider: FieldProvider;
    }
  | { kind: "edit_projection"; fieldId: string; other: OtherEntry; instanceId: string | null }
  | { kind: "remove_projection"; fieldId: string }
  | {
      kind: "create";
      name: string;
      fieldKind: FieldKind;
      unit?: string | null;
      provider: FieldProvider;
      options: string[];
      value: FieldValue | null;
    }
  | { kind: "set_hidden"; fieldId: string; hidden: boolean }
  | { kind: "set_values"; edits: { fieldId: string; value: FieldValue | null }[] }
  | { kind: "rename"; fieldId: string; name: string }
  | { kind: "set_retired"; fieldId: string; retired: boolean }
  | { kind: "bind" | "unbind"; fieldId: string; provider: FieldProvider }
  | { kind: "add_choice"; fieldId: string; label: string }
  | { kind: "rename_choice"; optionId: string; label: string }
  | { kind: "set_choice_retired"; optionId: string; retired: boolean };
export interface RelationshipDraft {
  name: string;
  forwardLabel: string;
  inverseLabel: string;
  directed: boolean;
  expectedTargetsPerSource: number | null;
  expectedSourcesPerTarget: number | null;
}
export interface RelationshipDefinition extends RelationshipDraft {
  id: string;
  retired: boolean;
  revision: number;
}
export interface RelationshipParticipant {
  id: string | null;
  label: string;
  workspaceState: string;
}
export interface Relationship {
  id: string;
  definitionId: string;
  source: RelationshipParticipant;
  target: RelationshipParticipant;
  note: string;
  ended: boolean;
  workspaceState: string;
  revision: number;
  warnings: string[];
}
export interface RelationshipSnapshot {
  globalRevision: number;
  definitions: RelationshipDefinition[];
  relationships: Relationship[];
  entries: { id: string; label: string; categoryName: string }[];
}
export type EntryRelationships = RelationshipSnapshot;
export type RelationshipCommand =
  | { kind: "create_definition"; draft: RelationshipDraft }
  | { kind: "update_definition"; definitionId: string; draft: RelationshipDraft }
  | { kind: "retire_definition"; definitionId: string; retired: boolean }
  | {
      kind: "connect";
      definitionId: string;
      perspective: "source" | "target";
      other: OtherEntry;
      note: string;
      replace: string[];
    }
  | { kind: "set_note"; id: string; note: string }
  | { kind: "set_ended"; id: string; ended: boolean };

export interface FieldCatalog {
  globalRevision: number;
  definitions: FieldDefinition[];
}

export interface FieldMergePreview {
  globalRevision: number;
  source: FieldDefinition;
  target: FieldDefinition;
  entries: {
    entryId: string;
    name: string;
    sourceValue: FieldValue | null;
    targetValue: FieldValue | null;
    conflict: boolean;
  }[];
  blockers: string[];
}
export interface FieldMergeOutcome {
  globalRevision: number;
  backupPath: string;
}

export interface EntryFieldDeleteOutcome {
  snapshot: EntryFields;
  backupPath: string;
}
export type CapabilityProvider = { kind: "category" | "type"; id: string };
export interface SpatialEntry {
  id: string;
  label: string;
  workspaceState: string;
  spatial: boolean;
  parentId: string | null;
}
export interface SpatialSnapshot {
  globalRevision: number;
  entries: SpatialEntry[];
  defaults: CapabilityProvider[];
}
export type SpatialCommand =
  | { kind: "set_enabled"; entryId: string; enabled: boolean }
  | { kind: "reparent"; entryId: string; parentId: string | null }
  | {
      kind: "create_child";
      parentId: string;
      name: string | null;
      categoryId: string | null;
      typeId: string | null;
    }
  | { kind: "set_default"; provider: CapabilityProvider; enabled: boolean };

export interface CategoryDeletePreview {
  globalRevision: number;
  name: string;
  entryCount: number;
  typedEntryCount: number;
  typeNames: string[];
  defaultCount: number;
}
export type StructureCommand =
  | { kind: "rename_category"; id: string; name: string }
  | { kind: "delete_category"; id: string; destinationId: string; removeTypes: boolean }
  | { kind: "set_entry_state"; id: string; state: "active" | "archived" | "trashed" };
export interface StructureOutcome {
  globalRevision: number;
  backupPath: string | null;
}
