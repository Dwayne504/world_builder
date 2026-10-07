/**
 * Typed wrappers around the Tauri commands exposed by `tauri_boundary`.
 * This is the only file in the frontend allowed to call `invoke`; every
 * other component goes through these functions.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AutomaticBackupStatus } from "./automaticBackupTypes";

export function getAutomaticBackupStatus(projectId: string): Promise<AutomaticBackupStatus> {
  return call("get_automatic_backup_status", { projectId });
}

export function setAutomaticBackupsEnabled(enabled: boolean): Promise<Preferences> {
  return call("set_automatic_backups_enabled", { enabled });
}
import type { TimelineCommand, TimelineSnapshot } from "./timelineTypes";
export function readTimeline(projectId: string): Promise<TimelineSnapshot> {
  return call("read_timeline", { projectId });
}
export function applyTimeline(
  projectId: string,
  expectedRevision: number,
  command: TimelineCommand,
): Promise<TimelineSnapshot> {
  return call("apply_timeline", { projectId, expectedRevision, command });
}

import type { ChapterSnapshot, StoryCommand, StoryIndex, StoryUsage } from "./storyTypes";

export function readStory(projectId: string): Promise<StoryIndex> {
  return call("read_story", { projectId });
}
export function readChapter(projectId: string, chapterId: string): Promise<ChapterSnapshot> {
  return call("read_chapter", { projectId, chapterId });
}
export function storyUsage(projectId: string, entryId: string): Promise<StoryUsage[]> {
  return call("story_usage", { projectId, entryId });
}
export function applyStory(
  projectId: string,
  expected: number,
  command: StoryCommand,
): Promise<ChapterSnapshot> {
  return call("apply_story", { projectId, expected, command });
}
import type {
  AppErrorDto,
  Category,
  Entry,
  Preferences,
  ProjectSummary,
  TypeDef,
  RecentProject,
} from "./types";
import type { EntryFields, FieldCommand, FieldCatalog } from "./types";
import type { EntryRelationships, RelationshipCommand, RelationshipSnapshot } from "./types";

export function readProjectRelationships(projectId: string): Promise<RelationshipSnapshot> {
  return call("read_project_relationships", { projectId });
}

export function readRelationships(projectId: string, entryId: string): Promise<EntryRelationships> {
  return call("read_relationships", { projectId, entryId });
}
export function applyRelationships(
  projectId: string,
  entryId: string,
  expectedRevision: number,
  command: RelationshipCommand,
): Promise<EntryRelationships> {
  return call("apply_relationships", { projectId, entryId, expectedRevision, command });
}

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

export function readFields(projectId: string, entryId: string): Promise<EntryFields> {
  return call("read_fields", { projectId, entryId });
}
export function applyFields(
  projectId: string,
  entryId: string,
  expectedRevision: number,
  command: FieldCommand,
): Promise<EntryFields> {
  return call("apply_fields", { projectId, entryId, expectedRevision, command });
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

export function listOpenProjects(): Promise<ProjectSummary[]> {
  return call("list_open_projects", {});
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

export function listEntries(
  projectId: string,
  workspaceState?: Entry["workspaceState"],
): Promise<Entry[]> {
  return call("list_entries", workspaceState ? { projectId, workspaceState } : { projectId });
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

export function getPreferences(): Promise<Preferences> {
  return call("get_preferences", {});
}

export function setDefaultProjectsDir(directory: string | null): Promise<Preferences> {
  return call("set_default_projects_dir", { directory });
}

export function setDefaultBackupsDir(directory: string | null): Promise<Preferences> {
  return call("set_default_backups_dir", { directory });
}

/** Explicit recovery from corrupt preferences; unsupported versions remain protected. */
export function resetPreferences(): Promise<Preferences> {
  return call("reset_preferences", {});
}

/**
 * Previews the exact package path `createProject` would use, computed by
 * the backend's authoritative sanitizer so the UI never maintains a
 * second, potentially diverging one.
 */
export function previewPackagePath(baseDir: string, workingName: string): Promise<string> {
  return call("preview_package_path", { baseDir, workingName });
}

/** Shows a native folder picker. Returns `null` if the user cancels. */
export function pickDirectory(defaultPath?: string | null): Promise<string | null> {
  return call("pick_directory", { defaultPath: defaultPath || null });
}

export function readFieldCatalog(projectId: string): Promise<FieldCatalog> {
  return call("read_field_catalog", { projectId });
}
export function applyTemplateFields(
  projectId: string,
  expectedRevision: number,
  command: FieldCommand,
): Promise<FieldCatalog> {
  return call("apply_template_fields", { projectId, expectedRevision, command });
}

export type Appearance = "storybook" | "starship";
export function getAppearance(): Promise<Appearance> {
  return call("get_appearance", {});
}
export function setAppearance(appearance: Appearance): Promise<Appearance> {
  return call("set_appearance", { appearance });
}

export function previewFieldMerge(
  projectId: string,
  sourceId: string,
  targetId: string,
): Promise<import("./types").FieldMergePreview> {
  return call("preview_field_merge", { projectId, sourceId, targetId });
}
export function mergeFields(
  projectId: string,
  sourceId: string,
  targetId: string,
  expectedRevision: number,
  backupDir: string,
): Promise<import("./types").FieldMergeOutcome> {
  return call("merge_fields", { projectId, sourceId, targetId, expectedRevision, backupDir });
}

export function deleteEntryField(
  projectId: string,
  entryId: string,
  fieldId: string,
  expectedRevision: number,
): Promise<import("./types").EntryFieldDeleteOutcome> {
  return call("delete_entry_field", { projectId, entryId, fieldId, expectedRevision });
}

export function automaticBackupDirectory(): Promise<string> {
  return call("automatic_backup_directory", {});
}

export function listRecentProjects(): Promise<RecentProject[]> {
  return call("list_recent_projects", {});
}
export function forgetRecentProject(projectId: string): Promise<void> {
  return call("forget_recent_project", { projectId });
}
export function openRecentProject(
  projectId: string,
  relocatedPath?: string,
  forceStaleLockRecovery = false,
): Promise<ProjectSummary> {
  return call("open_recent_project", { projectId, relocatedPath, forceStaleLockRecovery });
}
export function readSpatial(projectId: string): Promise<import("./types").SpatialSnapshot> {
  return call("read_spatial", { projectId });
}
export function applySpatial(
  projectId: string,
  expectedRevision: number,
  command: import("./types").SpatialCommand,
): Promise<import("./types").SpatialSnapshot> {
  return call("apply_spatial", { projectId, expectedRevision, command });
}

export function searchProject(
  projectId: string,
  request: import("./searchTypes").SearchRequest,
): Promise<import("./searchTypes").SearchResults> {
  return call("search_project", { projectId, request });
}
export function readAliases(
  projectId: string,
  entryId: string,
): Promise<import("./searchTypes").EntryAliases> {
  return call("read_aliases", { projectId, entryId });
}
export function applyAlias(
  projectId: string,
  entryId: string,
  expectedRevision: number,
  command: import("./searchTypes").AliasCommand,
): Promise<import("./searchTypes").EntryAliases> {
  return call("apply_alias", { projectId, entryId, expectedRevision, command });
}

export function previewCategoryDelete(
  projectId: string,
  categoryId: string,
): Promise<import("./types").CategoryDeletePreview> {
  return call("preview_category_delete", { projectId, categoryId });
}
export function applyStructure(
  projectId: string,
  expectedRevision: number,
  command: import("./types").StructureCommand,
): Promise<import("./types").StructureOutcome> {
  return call("apply_structure", { projectId, expectedRevision, command });
}

export interface BuildInfo {
  version: string;
  supportedSchemaVersion: number;
  supportedFormatVersion: number;
}

export function getBuildInfo(): Promise<BuildInfo> {
  return call("get_build_info", {});
}
