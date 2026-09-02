/**
 * Typed wrappers around the Tauri commands exposed by `tauri_boundary`.
 * This is the only file in the frontend allowed to call `invoke`; every
 * other component goes through these functions.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AppErrorDto, ProjectSummary } from "./types";

export class AppCommandError extends Error {
  kind: string;

  constructor(dto: AppErrorDto) {
    super(dto.message);
    this.kind = dto.kind;
  }
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    if (isAppErrorDto(err)) {
      throw new AppCommandError(err);
    }
    throw err;
  }
}

function isAppErrorDto(value: unknown): value is AppErrorDto {
  return typeof value === "object" && value !== null && "kind" in value && "message" in value;
}

export function createProject(baseDir: string, workingName: string): Promise<ProjectSummary> {
  return call("create_project", { baseDir, workingName });
}

export function openProject(
  packagePath: string,
  forceStaleLockRecovery = false,
): Promise<ProjectSummary> {
  return call("open_project", { packagePath, forceStaleLockRecovery });
}

export function renameProject(
  projectId: string,
  newName: string,
  expectedRevision: number,
): Promise<ProjectSummary> {
  return call("rename_project", { projectId, newName, expectedRevision });
}

export function closeProject(projectId: string): Promise<void> {
  return call("close_project", { projectId });
}

export function getProjectSummary(projectId: string): Promise<ProjectSummary> {
  return call("get_project_summary", { projectId });
}

export function createBackup(projectId: string, backupDir: string): Promise<string> {
  return call("create_backup", { projectId, backupDir });
}

export function restoreBackupAsCopy(
  backupPath: string,
  destinationDir: string,
  newWorkingName?: string,
): Promise<ProjectSummary> {
  return call("restore_backup_as_copy", {
    backupPath,
    destinationDir,
    newWorkingName: newWorkingName ?? null,
  });
}
