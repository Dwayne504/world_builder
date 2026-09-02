import { useCallback, useState } from "react";
import { renameProject } from "./api";
import type { ProjectSummary, SaveState } from "./types";

export interface UseProjectRenameResult {
  draftName: string;
  saveState: SaveState;
  errorMessage: string | null;
  committedName: string;
  revision: number;
  onChangeDraft: (value: string) => void;
  submit: () => Promise<void>;
  retry: () => Promise<void>;
  /** True while a rename is pending/saving/failed: closing must not discard this. */
  hasUnsavedWork: boolean;
}

/**
 * Drives the Saved-state contract for renaming a Project:
 *
 * 1. Typing marks the UI dirty/pending.
 * 2. Submitting sends the command to the Project worker ("saving").
 * 3. Only the worker's commit acknowledgement flips the UI to "saved".
 * 4. A failed commit keeps the unsaved draft value, reports the error, and
 *    stays out of "saved" until a successful retry.
 */
export function useProjectRename(project: ProjectSummary): UseProjectRenameResult {
  const [draftName, setDraftName] = useState(project.workingName);
  const [committedName, setCommittedName] = useState(project.workingName);
  const [revision, setRevision] = useState(project.revision);
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const onChangeDraft = useCallback(
    (value: string) => {
      setDraftName(value);
      setSaveState(value === committedName ? "saved" : "dirty");
      setErrorMessage(null);
    },
    [committedName],
  );

  const submit = useCallback(async () => {
    if (draftName === committedName) {
      return;
    }
    setSaveState("saving");
    setErrorMessage(null);
    try {
      const updated = await renameProject(project.projectId, draftName, revision);
      setCommittedName(updated.workingName);
      setRevision(updated.revision);
      setDraftName(updated.workingName);
      setSaveState("saved");
    } catch (err) {
      // The dirty draft value is intentionally retained: a failed save
      // must never silently claim "Saved" or discard what the user typed.
      setSaveState("failed");
      setErrorMessage(err instanceof Error ? err.message : "Failed to save.");
    }
  }, [draftName, committedName, project.projectId, revision]);

  const retry = submit;

  return {
    draftName,
    saveState,
    errorMessage,
    committedName,
    revision,
    onChangeDraft,
    submit,
    retry,
    hasUnsavedWork: saveState !== "saved",
  };
}
