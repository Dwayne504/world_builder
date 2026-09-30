/**
 * Wire types shared with the Tauri boundary (see
 * `src-tauri/src/tauri_boundary/dto.rs`). Kept intentionally small: this is
 * the Milestone 01 Trust Foundation slice, not the full domain model.
 */

export interface ProjectSummary {
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

export type FieldKind = "short_text" | "number" | "boolean" | "choice" | "multi_choice";
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
  retired: boolean;
  revision: number;
  options: ChoiceOption[];
  bindings: { provider: FieldProvider; label: string }[];
}
export type FieldValue =
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
}
export interface EntryFields {
  globalRevision: number;
  fields: EntryField[];
  definitions: FieldDefinition[];
}
export type FieldCommand =
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
export interface EntryRelationships {
  globalRevision: number;
  definitions: RelationshipDefinition[];
  relationships: Relationship[];
  entries: { id: string; label: string; categoryName: string }[];
}
export type RelationshipCommand =
  | { kind: "create_definition"; draft: RelationshipDraft }
  | { kind: "update_definition"; definitionId: string; draft: RelationshipDraft }
  | { kind: "retire_definition"; definitionId: string; retired: boolean }
  | {
      kind: "connect";
      definitionId: string;
      perspective: "source" | "target";
      other:
        | { kind: "existing"; id: string }
        | { kind: "create"; name: string | null; categoryId: string | null };
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
