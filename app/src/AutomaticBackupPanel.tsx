import type { AutomaticBackupStatus } from "./automaticBackupTypes";
import "./AutomaticBackupPanel.css";

export function AutomaticBackupToggle({
  enabled,
  disabled = false,
  onChange,
}: {
  enabled: boolean | null;
  disabled?: boolean;
  onChange: (enabled: boolean) => void;
}) {
  return (
    <label className="automatic-backup-toggle">
      <input
        type="checkbox"
        checked={enabled === true}
        disabled={disabled || enabled === null}
        onChange={(event) => onChange(event.target.checked)}
      />
      Automatic backups for open Projects
    </label>
  );
}

function dateLabel(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Time unavailable" : date.toLocaleString();
}

export function AutomaticBackupPanel({
  status,
  readError,
  actionError,
  changing,
  onChange,
  onRefresh,
}: {
  status: AutomaticBackupStatus | null;
  readError: string | null;
  actionError: string | null;
  changing: boolean;
  onChange: (enabled: boolean) => void;
  onRefresh: () => void;
}) {
  return (
    <section className="automatic-backup-panel" aria-label="Automatic backups">
      <h3>Automatic backups</h3>
      <AutomaticBackupToggle
        enabled={status?.enabled ?? null}
        disabled={changing || !!readError}
        onChange={onChange}
      />
      <p className="field-note">
        Changed Projects are backed up every {status?.intervalMinutes ?? 15} minutes while open. The
        newest {status?.retentionCount ?? 20} automatic copies per Project are kept. Manual and
        safety copies are kept separately.
      </p>
      {!status && !readError && <p role="status">Loading backup status…</p>}
      {status && (
        <>
          <p role="status">
            {readError
              ? "Automatic backup status is unavailable."
              : status.running
                ? "Creating an automatic backup…"
                : status.enabled === false
                  ? "Automatic backups are off."
                  : status.enabled === null
                    ? "Automatic backups are unavailable until preferences can be read."
                    : "Automatic backups are on."}
          </p>
          <dl className="automatic-backup-times">
            <div>
              <dt>Last automatic backup</dt>
              <dd>
                {status.lastSuccessAt ? (
                  <time dateTime={status.lastSuccessAt}>{dateLabel(status.lastSuccessAt)}</time>
                ) : (
                  "None this session"
                )}
              </dd>
            </div>
            {!readError && status.enabled && status.nextDueAt && (
              <div>
                <dt>Next check for changes</dt>
                <dd>
                  <time dateTime={status.nextDueAt}>{dateLabel(status.nextDueAt)}</time>
                </dd>
              </div>
            )}
          </dl>
        </>
      )}
      {(readError || status?.error || actionError) && (
        <div role="status" className="backup-warning">
          <p>{readError || status?.error || actionError}</p>
          {actionError && (readError || status?.error) && <p>{actionError}</p>}
          <p className="field-note">
            Backup problems do not stop you from writing. Autosave is separate.
          </p>
          <button className="quiet-button" disabled={changing} onClick={onRefresh}>
            Refresh backup status
          </button>
        </div>
      )}
      {(status?.backupDirectory || status?.lastSuccessPath) && (
        <details className="technical-details">
          <summary>Automatic backup location</summary>
          {status.backupDirectory && (
            <p className="package-preview">Folder: {status.backupDirectory}</p>
          )}
          {status.lastSuccessPath && (
            <p className="package-preview">Latest copy: {status.lastSuccessPath}</p>
          )}
          <p className="field-note">
            Restore a snapshot as a new Project from Home. Change the default Backups folder in
            Application settings.
          </p>
        </details>
      )}
    </section>
  );
}
