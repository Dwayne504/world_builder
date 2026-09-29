import { useEffect, useRef, useState } from "react";
import { getPreferences, pickDirectory } from "./api";
import { Dialog } from "./Dialog";
import type { EntryField } from "./types";

export function DeleteEntryFieldDialog({
  field,
  entryName,
  busy,
  error,
  onClose,
  onDelete,
}: {
  field: EntryField;
  entryName: string;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onDelete: (backupDir: string) => void;
}) {
  const [backupDir, setBackupDir] = useState("");
  const [picking, setPicking] = useState(false);
  const [folderError, setFolderError] = useState<string | null>(null);
  const touched = useRef(false);
  useEffect(() => {
    let alive = true;
    void getPreferences()
      .then((prefs) => {
        if (alive && !touched.current && prefs.defaultBackupsDirExists)
          setBackupDir(prefs.defaultBackupsDir ?? "");
      })
      .catch(() => {
        /* An explicit destination is still required. */
      });
    return () => {
      alive = false;
    };
  }, []);
  async function choose() {
    setPicking(true);
    setFolderError(null);
    try {
      const path = await pickDirectory(backupDir || null);
      if (path) {
        touched.current = true;
        setBackupDir(path);
      }
    } catch (reason) {
      setFolderError(reason instanceof Error ? reason.message : "The folder could not be chosen.");
    } finally {
      setPicking(false);
    }
  }
  const value = field.value;
  const description =
    value?.kind === "choices"
      ? value.value
          .map((id) => field.definition.options.find((o) => o.id === id)?.label ?? "Unknown option")
          .join(", ")
      : value?.kind === "boolean"
        ? value.value
          ? "Yes"
          : "No"
        : value
          ? String(value.value)
          : "Not filled in";
  return (
    <Dialog open title={`Delete ${field.definition.name} from this Entry?`} onClose={onClose}>
      <p>
        This removes <strong>{field.definition.name}</strong> and its value from{" "}
        <strong>{entryName}</strong> only. Other Entries and shared defaults stay unchanged.
      </p>
      <p className="delete-value-preview">
        Current value: {description}
        {value && field.definition.unit ? ` ${field.definition.unit}` : ""}
      </p>
      <p>
        You can add the Field back empty later. To keep its current value out of sight, use Hide
        instead.
      </p>
      <fieldset disabled={busy || picking}>
        <label>
          Recovery backup folder
          <input
            aria-label="Delete Field backup folder"
            value={backupDir}
            onChange={(e) => {
              touched.current = true;
              setBackupDir(e.target.value);
            }}
          />
        </label>
        <button onClick={() => void choose()}>Choose backup folder…</button>
        <p className="field-note">
          A backup must succeed first. Restore it as a copy to recover the deleted value.
        </p>
        <div className="row">
          <button disabled={!backupDir.trim()} onClick={() => onDelete(backupDir)}>
            Back up and delete from Entry
          </button>
          <button onClick={onClose}>Cancel</button>
        </div>
      </fieldset>
      {(error || folderError) && <p role="alert">{error || folderError}</p>}
      {busy && <p role="status">Saving recovery backup and deleting…</p>}
    </Dialog>
  );
}
