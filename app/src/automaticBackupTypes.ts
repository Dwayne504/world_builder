export interface AutomaticBackupStatus {
  enabled: boolean | null;
  intervalMinutes: number;
  retentionCount: number;
  running: boolean;
  lastSuccessAt: string | null;
  lastSuccessRevision: number | null;
  nextDueAt: string | null;
  error: string | null;
  backupDirectory: string | null;
  lastSuccessPath: string | null;
}
