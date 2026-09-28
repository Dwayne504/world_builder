import { useCallback, useEffect, useRef, useState } from "react";
import { applyFields, readFields } from "./api";
import type { EntryField, EntryFields, FieldCommand, FieldValue, SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export type FieldDraft = string | string[];
export function valueDraft(value: FieldValue | null): FieldDraft {
  if (!value) return "";
  return value.kind === "choices" ? [...value.value] : String(value.value);
}
export function parseFieldDraft(field: EntryField, draft: FieldDraft): FieldValue | null {
  switch (field.definition.kind) {
    case "short_text":
      return draft === "" ? null : { kind: "text", value: String(draft) };
    case "number": {
      if (String(draft).trim() === "") return null;
      const value = Number(draft);
      if (!Number.isFinite(value))
        throw new Error(`${field.definition.name}: enter a finite number or leave it empty.`);
      return { kind: "number", value };
    }
    case "boolean":
      return draft === "" ? null : { kind: "boolean", value: draft === "true" };
    default: {
      const ids = Array.isArray(draft) ? draft : draft ? [draft] : [];
      return ids.length ? { kind: "choices", value: ids } : null;
    }
  }
}

export function useEntryFields(
  projectId: string,
  entryId: string,
  entryRevision: number,
  onRevision: (revision: number) => void,
) {
  const [snapshot, setSnapshot] = useState<EntryFields | null>(null);
  const [drafts, setDrafts] = useState<Record<string, FieldDraft>>({});
  const [state, setState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const snapshotRef = useRef(snapshot);
  const draftsRef = useRef(drafts);
  const revisionRef = useRef(onRevision);
  revisionRef.current = onRevision;
  const inFlight = useRef<Promise<SubmitOutcome> | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const generation = useRef(0);
  const mounted = useRef(true);
  const accept = useCallback((updated: EntryFields) => {
    snapshotRef.current = updated;
    setSnapshot(updated);
    revisionRef.current(updated.globalRevision);
  }, []);
  const reload = useCallback(async () => {
    const request = ++generation.current;
    try {
      const updated = await readFields(projectId, entryId);
      if (!mounted.current || request !== generation.current) return;
      accept(updated);
      setError(null);
      setState(Object.keys(draftsRef.current).length ? "dirty" : "saved");
    } catch (err) {
      if (!mounted.current || request !== generation.current) return;
      setError(err instanceof Error ? err.message : "Fields could not be loaded.");
      setState("failed");
    }
  }, [projectId, entryId, accept]);
  useEffect(() => {
    mounted.current = true;
    void reload();
    return () => {
      mounted.current = false;
      if (timer.current) clearTimeout(timer.current);
    };
  }, [reload]);
  const previousEntryRevision = useRef(entryRevision);
  useEffect(() => {
    if (previousEntryRevision.current === entryRevision) return;
    previousEntryRevision.current = entryRevision;
    // Never silently rebase unsaved field edits onto newer authoritative data.
    if (!inFlight.current && !Object.keys(draftsRef.current).length) void reload();
  }, [entryRevision, reload]);

  const command = useCallback(
    (operation: FieldCommand, afterCommit?: () => SubmitOutcome): Promise<SubmitOutcome> => {
      if (inFlight.current) return inFlight.current;
      if (!snapshotRef.current) return Promise.resolve({ kind: "failed" });
      ++generation.current;
      setState("saving");
      setError(null);
      const request = applyFields(projectId, entryId, snapshotRef.current.globalRevision, operation)
        .then((updated): SubmitOutcome => {
          accept(updated);
          const outcome: SubmitOutcome = afterCommit?.() ?? { kind: "committed" };
          setState(outcome.kind === "committed-stale" ? "dirty" : "saved");
          return outcome;
        })
        .catch((err: unknown): SubmitOutcome => {
          setState("failed");
          setError(err instanceof Error ? err.message : "Fields could not be saved.");
          return { kind: "failed" };
        })
        .finally(() => {
          inFlight.current = null;
        });
      inFlight.current = request;
      return request;
    },
    [projectId, entryId, accept],
  );

  const submit = useCallback((): Promise<SubmitOutcome> => {
    if (timer.current) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    if (inFlight.current) return inFlight.current;
    const submitted = { ...draftsRef.current };
    if (!Object.keys(submitted).length) return Promise.resolve({ kind: "no-op" });
    const current = snapshotRef.current;
    if (!current) return Promise.resolve({ kind: "failed" });
    let edits;
    try {
      edits = Object.entries(submitted).map(([fieldId, draft]) => {
        const field = current.fields.find((f) => f.definition.id === fieldId);
        if (!field)
          throw new Error("An edited field is no longer available. Your draft is preserved.");
        return { fieldId, value: parseFieldDraft(field, draft) };
      });
    } catch (err) {
      setState("failed");
      setError((err as Error).message);
      return Promise.resolve({ kind: "failed" });
    }
    // Inputs are disabled during publication. Drafts are cleared only after ack.
    return command({ kind: "set_values", edits }, () => {
      const remaining = Object.fromEntries(
        Object.entries(draftsRef.current).filter(
          ([id, value]) => JSON.stringify(value) !== JSON.stringify(submitted[id]),
        ),
      );
      draftsRef.current = remaining;
      setDrafts(remaining);
      if (Object.keys(remaining).length) timer.current = setTimeout(() => void submit(), 500);
      return { kind: Object.keys(remaining).length ? "committed-stale" : "committed" };
    });
  }, [command]);
  const change = useCallback(
    (fieldId: string, draft: FieldDraft) => {
      draftsRef.current = { ...draftsRef.current, [fieldId]: draft };
      setDrafts(draftsRef.current);
      setState("dirty");
      setError(null);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        timer.current = null;
        void submit();
      }, 500);
    },
    [submit],
  );
  return { snapshot, drafts, state, error, reload, command, submit, change };
}
