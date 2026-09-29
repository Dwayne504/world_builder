import { useCallback, useEffect, useRef, useState } from "react";
import { getPreferences, mergeFields, pickDirectory, previewFieldMerge } from "./api";
import type { FieldDefinition, FieldMergeOutcome, FieldMergePreview, FieldValue } from "./types";
import { fieldLabel, matchingFields } from "./fieldLabels";

function display(value: FieldValue | null) {
  if (!value) return "Not filled in";
  if (value.kind === "boolean") return value.value ? "Yes" : "No";
  if (value.kind === "choices") return "Choice selection";
  return String(value.value);
}

export function FieldMergeReview({
  projectId,
  definitions,
  disabled,
  onCommit,
}: {
  projectId: string;
  definitions: FieldDefinition[];
  disabled: boolean;
  onCommit: (action: () => Promise<FieldMergeOutcome>) => void;
}) {
  const [targetId, setTargetId] = useState("");
  const [sourceId, setSourceId] = useState("");
  const [preview, setPreview] = useState<FieldMergePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [backupDir, setBackupDir] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const generation = useRef(0);
  const touched = useRef(false);
  const invalidate = useCallback(() => {
    ++generation.current;
  }, []);
  useEffect(() => {
    let current = true;
    void getPreferences()
      .then((prefs) => {
        if (current && !touched.current && prefs.defaultBackupsDirExists)
          setBackupDir(prefs.defaultBackupsDir ?? "");
      })
      .catch(() => {
        /* A chosen destination is required; no silent fallback. */
      });
    return () => {
      current = false;
      invalidate();
    };
  }, [invalidate]);
  const target = definitions.find((d) => d.id === targetId);
  const candidates = target
    ? matchingFields(definitions, target.name).filter((d) => d.id !== targetId)
    : [];
  function clearPreview() {
    ++generation.current;
    setPreview(null);
    setConfirmed(false);
    setError(null);
  }
  async function review() {
    const request = ++generation.current;
    setBusy(true);
    setPreview(null);
    setConfirmed(false);
    setError(null);
    try {
      const result = await previewFieldMerge(projectId, sourceId, targetId);
      if (request === generation.current) setPreview(result);
    } catch (reason) {
      if (request === generation.current)
        setError(reason instanceof Error ? reason.message : "The merge could not be reviewed.");
    } finally {
      if (request === generation.current) setBusy(false);
    }
  }
  async function chooseBackup() {
    setBusy(true);
    try {
      const path = await pickDirectory(backupDir || null);
      if (path) {
        touched.current = true;
        setBackupDir(path);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The folder could not be chosen.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="field-merge-review">
      <p>
        Combine two same-name Fields across this Project. Choose the definition to keep; its
        identity stays the same.
      </p>
      <fieldset disabled={disabled || busy}>
        <label>
          Keep this Field
          <select
            aria-label="Keep Field"
            value={targetId}
            onChange={(e) => {
              setTargetId(e.target.value);
              setSourceId("");
              clearPreview();
            }}
          >
            <option value="">Choose a Field</option>
            {definitions
              .filter((d) => !d.retired && matchingFields(definitions, d.name).length > 1)
              .map((d) => (
                <option key={d.id} value={d.id}>
                  {fieldLabel(d, definitions)}
                </option>
              ))}
          </select>
        </label>
        <label>
          Combine this duplicate into it
          <select
            aria-label="Duplicate Field"
            value={sourceId}
            onChange={(e) => {
              setSourceId(e.target.value);
              clearPreview();
            }}
          >
            <option value="">Choose a duplicate</option>
            {candidates.map((d) => (
              <option key={d.id} value={d.id}>
                {fieldLabel(d, definitions)}
              </option>
            ))}
          </select>
        </label>
        <button disabled={!targetId || !sourceId} onClick={() => void review()}>
          Review merge
        </button>
      </fieldset>
      {busy && <p role="status">Preparing review…</p>}
      {error && <p role="alert">{error}</p>}
      {preview && (
        <>
          <h3>Review: {preview.target.name}</h3>
          <p>Keep: {fieldLabel(preview.target, definitions)}</p>
          <p>Remove duplicate: {fieldLabel(preview.source, definitions)}</p>
          <p>
            Both Fields' Category, Type, and Entry defaults will use the kept definition. Missing
            values are filled from the duplicate; identical values become one. The duplicate
            definition is removed. Existing Hide or Delete choices on the kept Field take
            precedence. Otherwise, the duplicate's local visibility choice is kept.
          </p>
          <p>
            {preview.entries.length} affected Entries, including empty Fields and retained
            historical values.
          </p>
          <div className="merge-table-scroll">
            <table className="merge-table">
              <thead>
                <tr>
                  <th>Entry</th>
                  <th>Kept Field</th>
                  <th>Duplicate</th>
                  <th>Result</th>
                </tr>
              </thead>
              <tbody>
                {preview.entries.map((e) => (
                  <tr key={e.entryId}>
                    <th>{e.name}</th>
                    <td>{display(e.targetValue)}</td>
                    <td>{display(e.sourceValue)}</td>
                    <td>
                      {e.conflict
                        ? "Conflict — merge blocked"
                        : display(e.targetValue ?? e.sourceValue)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!!preview.blockers.length && (
            <div role="alert" className="error-banner">
              {preview.blockers.map((b) => (
                <p key={b}>{b}</p>
              ))}
            </div>
          )}
          <fieldset disabled={disabled || busy || !!preview.blockers.length}>
            <label>
              Recovery backup folder
              <input
                aria-label="Merge backup folder"
                value={backupDir}
                onChange={(e) => {
                  touched.current = true;
                  setBackupDir(e.target.value);
                }}
              />
            </label>
            <button onClick={() => void chooseBackup()}>Choose backup folder…</button>
            <p className="field-note">
              A verified snapshot is created here before anything changes. Restore it as a copy from
              the opening screen if you need to undo the merge.
            </p>
            <label className="merge-confirm">
              <input
                type="checkbox"
                checked={confirmed}
                onChange={(e) => setConfirmed(e.target.checked)}
              />
              I reviewed the values and combined defaults.
            </label>
            <button
              disabled={!confirmed || !backupDir.trim()}
              onClick={() => {
                setConfirmed(false);
                onCommit(() =>
                  mergeFields(
                    projectId,
                    preview.source.id,
                    preview.target.id,
                    preview.globalRevision,
                    backupDir,
                  ),
                );
              }}
            >
              Back up and merge
            </button>
          </fieldset>
        </>
      )}
    </div>
  );
}
