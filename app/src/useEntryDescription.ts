import { useCallback, useEffect, useRef, useState } from "react";
import type { JSONContent } from "@tiptap/react";
import { readEntryDescription, saveEntryDescription } from "./api";
import {
  readEntryDescriptionDraft,
  storeEntryDescriptionDraft,
  type EntryDescriptionRecovery,
} from "./entryDescriptionRecovery";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";
import type { SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export function useEntryDescription(
  projectId: string,
  entryId: string,
  onRevision: (revision: number) => void,
  getRevision: () => number,
) {
  const [snapshot, setSnapshot] = useState<EntryDescriptionSnapshot | null>(null);
  const [draft, setDraft] = useState<JSONContent | null>(null);
  const [recovery, setRecovery] = useState<EntryDescriptionRecovery | null>(null);
  const [state, setState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [recoveryAvailable, setRecoveryAvailable] = useState(true);
  const [editorEpoch, setEditorEpoch] = useState(0);
  const current = useRef(snapshot);
  const drafts = useRef(draft);
  const recovered = useRef(recovery);
  const failed = useRef(false);
  const reviewingRef = useRef(false);
  const mounted = useRef(true);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const attemptedRevision = useRef<number | null>(null);
  const callbacks = useRef({ onRevision, getRevision });
  callbacks.current = { onRevision, getRevision };
  const accept = useCallback((next: EntryDescriptionSnapshot) => {
    current.current = next;
    setSnapshot(next);
    callbacks.current.onRevision(next.globalRevision);
  }, []);
  const persist = useCallback(() => {
    setRecoveryAvailable(
      storeEntryDescriptionDraft(
        projectId,
        entryId,
        current.current?.globalRevision ?? 0,
        drafts.current,
      ),
    );
  }, [projectId, entryId]);

  useEffect(() => {
    let active = true;
    mounted.current = true;
    void readEntryDescription(projectId, entryId)
      .then((next) => {
        if (!active) return;
        accept(next);
        const held = readEntryDescriptionDraft(projectId, next);
        recovered.current = held;
        setRecovery(held);
        setState(held ? "dirty" : "saved");
      })
      .catch((reason: unknown) => {
        if (active)
          setError(reason instanceof Error ? reason.message : "Description could not be loaded.");
      });
    return () => {
      active = false;
      mounted.current = false;
      if (timer.current) clearTimeout(timer.current);
    };
  }, [projectId, entryId, accept]);

  const save = useCallback(
    (retry = false): Promise<SubmitOutcome> => {
      if (timer.current) clearTimeout(timer.current);
      if (pending.current) return pending.current;
      if (recovered.current || reviewingRef.current || (failed.current && !retry))
        return Promise.resolve({ kind: "failed" });
      const submitted = drafts.current;
      if (!submitted) return Promise.resolve({ kind: "no-op" });
      const saved = current.current;
      if (!saved || saved.workspaceState !== "active" || saved.document?.readOnlyReason)
        return Promise.resolve({ kind: "failed" });
      // A retry retains the failed revision. Only explicit review permits rebasing a draft.
      const expected =
        failed.current && attemptedRevision.current !== null
          ? attemptedRevision.current
          : Math.max(saved.globalRevision, callbacks.current.getRevision());
      attemptedRevision.current = expected;
      setState("saving");
      setError(null);
      // Companion edits may advance the global revision. The independent
      // document revision still protects the writing loaded by this editor.
      const request = saveEntryDescription(
        projectId,
        entryId,
        expected,
        saved.document?.revision ?? null,
        1,
        submitted,
      )
        .then((next): SubmitOutcome => {
          accept(next);
          failed.current = false;
          if (JSON.stringify(drafts.current) === JSON.stringify(submitted)) {
            drafts.current = null;
            setDraft(null);
          }
          persist();
          setState(drafts.current ? "dirty" : "saved");
          return { kind: drafts.current ? "committed-stale" : "committed" };
        })
        .catch((reason: unknown): SubmitOutcome => {
          failed.current = true;
          setState("failed");
          setError(reason instanceof Error ? reason.message : "Description could not be saved.");
          persist();
          return { kind: "failed" };
        })
        .finally(() => {
          pending.current = null;
        });
      pending.current = request;
      return request;
    },
    [projectId, entryId, accept, persist],
  );
  const flush = useCallback(async (): Promise<SubmitOutcome> => {
    let result = await save();
    while (result.kind === "committed-stale") result = await save();
    return result;
  }, [save]);
  const retry = useCallback(async (): Promise<SubmitOutcome> => {
    const result = await save(true);
    return result.kind === "committed-stale" ? flush() : result;
  }, [save, flush]);
  useEffect(() => {
    if (state !== "dirty" || recovered.current || failed.current || !draft) return;
    timer.current = setTimeout(() => void save(), 350);
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, [draft, state, save]);

  const change = useCallback(
    (content: JSONContent) => {
      if (recovered.current || reviewingRef.current) return;
      drafts.current = content;
      setDraft(content);
      persist();
      setState(failed.current ? "failed" : pending.current ? "saving" : "dirty");
    },
    [persist],
  );
  const reviewSaved = useCallback(async () => {
    if (pending.current || reviewingRef.current) return;
    reviewingRef.current = true;
    setReviewing(true);
    if (timer.current) clearTimeout(timer.current);
    try {
      const next = await readEntryDescription(projectId, entryId);
      if (!mounted.current) return;
      const held: EntryDescriptionRecovery | null = drafts.current
        ? {
            version: 1,
            revision: current.current?.globalRevision ?? 0,
            savedAt: new Date().toISOString(),
            content: drafts.current,
          }
        : readEntryDescriptionDraft(projectId, next);
      accept(next);
      recovered.current = held;
      setRecovery(held);
      drafts.current = null;
      setDraft(null);
      failed.current = false;
      setError(null);
      setState(held ? "dirty" : "saved");
      setEditorEpoch((value) => value + 1);
    } catch (reason) {
      if (mounted.current)
        setError(
          reason instanceof Error ? reason.message : "Saved description could not be loaded.",
        );
    } finally {
      reviewingRef.current = false;
      if (mounted.current) setReviewing(false);
    }
  }, [projectId, entryId, accept]);
  const useRecovered = useCallback(() => {
    const held = recovered.current;
    if (
      !held?.content ||
      held.readOnlyReason ||
      current.current?.workspaceState !== "active" ||
      current.current.document?.readOnlyReason
    )
      return;
    recovered.current = null;
    setRecovery(null);
    change(held.content);
    setEditorEpoch((value) => value + 1);
  }, [change]);
  const keepSaved = useCallback(() => {
    recovered.current = null;
    setRecovery(null);
    drafts.current = null;
    setDraft(null);
    setState("saved");
    persist();
    setEditorEpoch((value) => value + 1);
  }, [persist]);
  return {
    snapshot,
    draft,
    recovery,
    state,
    error,
    reviewing,
    recoveryAvailable,
    editorEpoch,
    change,
    flush,
    retry,
    reviewSaved,
    useRecovered,
    keepSaved,
  };
}
