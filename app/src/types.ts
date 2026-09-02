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
