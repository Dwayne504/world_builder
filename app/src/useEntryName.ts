import { useCallback, useEffect, useRef, useState } from "react";
import { updateEntryName } from "./api";
import type { Entry, SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export function useEntryName(projectId: string, initialEntry: Entry) {
  const [entry, setEntry] = useState(initialEntry);
  const [draftName, setDraftName] = useState(initialEntry.authoredName ?? "");
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const draftRef = useRef(draftName);
  const committedRef = useRef(draftName);
  const entryRef = useRef(initialEntry);
  const inFlightRef = useRef<Promise<SubmitOutcome> | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  entryRef.current = entry;

  const submit = useCallback((): Promise<SubmitOutcome> => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (inFlightRef.current) return inFlightRef.current;
    const submittedName = draftRef.current;
    if (submittedName === committedRef.current) return Promise.resolve({ kind: "no-op" });
    setSaveState("saving");
    setErrorMessage(null);
    const request = updateEntryName(
      projectId,
      entryRef.current.id,
      submittedName,
      entryRef.current.revision,
    )
      .then((updated): SubmitOutcome => {
        entryRef.current = updated;
        committedRef.current = updated.authoredName ?? "";
        setEntry(updated);
        if (draftRef.current === submittedName) {
          setDraftName(updated.authoredName ?? "");
          setSaveState("saved");
          return { kind: "committed" };
        }
        setSaveState("dirty");
        timerRef.current = setTimeout(() => void submit(), 500);
        return { kind: "committed-stale" };
      })
      .catch((error: unknown): SubmitOutcome => {
        setSaveState("failed");
        setErrorMessage(error instanceof Error ? error.message : "Failed to save.");
        return { kind: "failed" };
      })
      .finally(() => {
        inFlightRef.current = null;
      });
    inFlightRef.current = request;
    return request;
  }, [projectId]);

  const onChangeDraft = useCallback(
    (value: string) => {
      draftRef.current = value;
      setDraftName(value);
      setErrorMessage(null);
      setSaveState(value === committedRef.current ? "saved" : "dirty");
      if (timerRef.current) clearTimeout(timerRef.current);
      if (value !== committedRef.current) {
        timerRef.current = setTimeout(() => void submit(), 500);
      }
    },
    [submit],
  );

  useEffect(
    () => () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    },
    [],
  );

  const replaceEntry = useCallback(
    (updated: Entry) => {
      const hasNewerDraft = draftRef.current !== committedRef.current;
      entryRef.current = updated;
      committedRef.current = updated.authoredName ?? "";
      setEntry(updated);
      if (hasNewerDraft) {
        setSaveState("dirty");
        if (timerRef.current) clearTimeout(timerRef.current);
        timerRef.current = setTimeout(() => void submit(), 500);
      } else {
        draftRef.current = updated.authoredName ?? "";
        setDraftName(updated.authoredName ?? "");
        setSaveState("saved");
      }
      setErrorMessage(null);
    },
    [submit],
  );

  const currentEntry = useCallback(() => entryRef.current, []);

  return {
    entry,
    draftName,
    saveState,
    errorMessage,
    onChangeDraft,
    submit,
    replaceEntry,
    currentEntry,
  };
}
