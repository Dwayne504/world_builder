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
