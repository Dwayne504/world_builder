/**
 * Typed wrappers around the Tauri commands exposed by `tauri_boundary`.
 * This is the only file in the frontend allowed to call `invoke`; every
 * other component goes through these functions.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AppErrorDto, Category, Entry, ProjectSummary, TypeDef } from "./types";

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

export function listCategories(projectId: string): Promise<Category[]> {
  return call("list_categories", { projectId });
}

export function createCategory(projectId: string, name: string): Promise<Category> {
  return call("create_category", { projectId, name });
}

export function listTypes(projectId: string, categoryId: string): Promise<TypeDef[]> {
  return call("list_types", { projectId, categoryId });
}

export function createType(
  projectId: string,
  categoryId: string,
  name: string,
  parentTypeId?: string,
): Promise<TypeDef> {
  return call("create_type", { projectId, categoryId, name, parentTypeId: parentTypeId ?? null });
}

export function listEntries(projectId: string): Promise<Entry[]> {
  return call("list_entries", { projectId });
}

export function createEntry(
  projectId: string,
  authoredName?: string,
  categoryId?: string,
  typeId?: string,
): Promise<Entry> {
  return call("create_entry", {
    projectId,
    authoredName: authoredName || null,
    categoryId: categoryId || null,
    typeId: typeId || null,
  });
}

export function getEntry(projectId: string, entryId: string): Promise<Entry> {
  return call("get_entry", { projectId, entryId });
}

export function updateEntryName(
  projectId: string,
  entryId: string,
  authoredName: string,
  expectedRevision: number,
): Promise<Entry> {
  return call("update_entry_name", {
    projectId,
    entryId,
    authoredName: authoredName || null,
    expectedRevision,
  });
}

export function changeEntryStructure(
  projectId: string,
  entryId: string,
  categoryId: string,
  typeId: string | undefined,
  expectedRevision: number,
): Promise<Entry> {
  return call("change_entry_structure", {
    projectId,
    entryId,
    categoryId,
    typeId: typeId || null,
    expectedRevision,
  });
}
