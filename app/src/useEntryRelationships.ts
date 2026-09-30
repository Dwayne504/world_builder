import { useCallback, useEffect, useRef, useState } from "react";
import { applyRelationships, readRelationships } from "./api";
import type { EntryRelationships, RelationshipCommand, SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export function useEntryRelationships(
  projectId: string,
  entryId: string,
  onRevision: (revision: number) => void,
  getRevision: () => number,
  onCommitted?: (revision: number) => void,
) {
  const [snapshot, setSnapshot] = useState<EntryRelationships | null>(null);
  const [state, setState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const snapshotRef = useRef(snapshot);
  const callbacks = useRef({ onRevision, getRevision, onCommitted });
  callbacks.current = { onRevision, getRevision, onCommitted };
  const inFlight = useRef<Promise<SubmitOutcome> | null>(null);
  const generation = useRef(0);
  const accept = useCallback((updated: EntryRelationships) => {
    snapshotRef.current = updated;
    setSnapshot(updated);
    callbacks.current.onRevision(updated.globalRevision);
  }, []);
  const reload = useCallback(async () => {
    const request = ++generation.current;
    try {
      const updated = await readRelationships(projectId, entryId);
      if (request !== generation.current) return;
      accept(updated);
      setError(null);
      setState("saved");
    } catch (err) {
      if (request !== generation.current) return;
      setError(err instanceof Error ? err.message : "Relationships could not be loaded.");
      setState("failed");
    }
  }, [projectId, entryId, accept]);
  const invalidate = useCallback(() => {
    ++generation.current;
  }, []);
  useEffect(() => {
    void reload();
    return invalidate;
  }, [reload, invalidate]);
  const command = useCallback(
    (operation: RelationshipCommand): Promise<SubmitOutcome> => {
      if (inFlight.current) return inFlight.current;
      if (!snapshotRef.current) return Promise.resolve({ kind: "failed" });
      ++generation.current;
      setState("saving");
      setError(null);
      // Only acknowledged writes in this editor advance this revision. External
      // writes still fail the backend comparison and require an explicit reload.
      const expected = Math.max(
        snapshotRef.current.globalRevision,
        callbacks.current.getRevision(),
      );
      const request = applyRelationships(projectId, entryId, expected, operation)
        .then((updated): SubmitOutcome => {
          accept(updated);
          callbacks.current.onCommitted?.(updated.globalRevision);
          setState("saved");
          return { kind: "committed" };
        })
        .catch((err: unknown): SubmitOutcome => {
          setError(err instanceof Error ? err.message : "Relationship could not be saved.");
          setState("failed");
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
  const wait = useCallback(
    () => inFlight.current ?? Promise.resolve({ kind: "no-op" } as SubmitOutcome),
    [],
  );
  return { snapshot, state, error, reload, command, wait };
}
