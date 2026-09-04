import { useCallback, useRef, useState } from "react";
import { renameProject } from "./api";
import type { ProjectSummary, SaveState } from "./types";

export interface UseProjectRenameResult {
  draftName: string;
  saveState: SaveState;
  errorMessage: string | null;
  committedName: string;
  revision: number;
  onChangeDraft: (value: string) => void;
  submit: () => Promise<SubmitOutcome>;
  retry: () => Promise<SubmitOutcome>;
  /** True while a rename is pending/saving/failed: closing must not discard this. */
  hasUnsavedWork: boolean;
}

/**
 * Explicit, control-flow-friendly result of a `submit()` call. Callers that
 * need to know whether it is safe to proceed (e.g. completing a pending
 * close) must branch on this value rather than re-reading `saveState`
 * afterwards: React state updates are not guaranteed to have committed by
 * the time a `.then()` continuation on the same promise runs, so reading
 * state there is a race, not a reliable signal.
 */
export type SubmitOutcome =
  { kind: "no-op" } | { kind: "committed" } | { kind: "committed-stale" } | { kind: "failed" };

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
  const draftRef = useRef(draftName);
  const committedNameRef = useRef(committedName);
  const revisionRef = useRef(revision);
  const inFlightRef = useRef<Promise<SubmitOutcome> | null>(null);

  const onChangeDraft = useCallback((value: string) => {
    draftRef.current = value;
    setDraftName(value);
    setSaveState(value === committedNameRef.current ? "saved" : "dirty");
    setErrorMessage(null);
  }, []);

  const submit = useCallback((): Promise<SubmitOutcome> => {
    if (inFlightRef.current) {
      return inFlightRef.current;
    }
    const submittedName = draftRef.current;
    const submittedRevision = revisionRef.current;
    if (submittedName === committedNameRef.current) {
      return Promise.resolve({ kind: "no-op" });
    }
    setSaveState("saving");
    setErrorMessage(null);
    const request = renameProject(project.projectId, submittedName, submittedRevision)
      .then((updated): SubmitOutcome => {
        committedNameRef.current = updated.workingName;
        revisionRef.current = updated.revision;
        setCommittedName(updated.workingName);
        setRevision(updated.revision);
        if (draftRef.current === submittedName) {
          setDraftName(updated.workingName);
          setSaveState("saved");
          return { kind: "committed" };
        }
        // A newer draft must remain pending even though this older submit
        // committed successfully -- the currently displayed draft was never
        // saved, so callers must not treat this as "safe to close".
        setSaveState("dirty");
        return { kind: "committed-stale" };
      })
      .catch((err: unknown): SubmitOutcome => {
        // A newer draft must remain pending even when the older save failed.
        setSaveState("failed");
        setErrorMessage(err instanceof Error ? err.message : "Failed to save.");
        return { kind: "failed" };
      })
      .finally(() => {
        inFlightRef.current = null;
      });
    inFlightRef.current = request;
    return request;
  }, [project.projectId]);

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
