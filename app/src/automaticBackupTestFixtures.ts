import type { AutomaticBackupStatus } from "./automaticBackupTypes";
import type { Preferences } from "./types";

export const backupPreferences = (enabled = true): Preferences => ({
  automaticBackupsEnabled: enabled,
  defaultProjectsDir: null,
  defaultProjectsDirExists: false,
  defaultBackupsDir: null,
  defaultBackupsDirExists: false,
});

export const automaticBackupFixture = (
  overrides: Partial<AutomaticBackupStatus> = {},
): AutomaticBackupStatus => ({
  enabled: true,
  intervalMinutes: 15,
  retentionCount: 20,
  running: false,
  lastSuccessAt: null,
  lastSuccessRevision: null,
  nextDueAt: "2026-01-02T12:15:00Z",
  error: null,
  backupDirectory: "/automatic-backups",
  lastSuccessPath: null,
  ...overrides,
});
