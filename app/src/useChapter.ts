import { useCallback, useEffect, useRef, useState } from "react";
import { applyStory, readChapter } from "./api";
import { storeChapterDraft } from "./chapterRecovery";
import type { ChapterDraft, ChapterSnapshot, DocumentArea, StoryCommand } from "./storyTypes";
import type { SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";
import type { JSONContent } from "@tiptap/react";

const empty = (): ChapterDraft => ({ documents: {} });
const dirty = (draft: ChapterDraft) =>
  draft.title !== undefined || Object.keys(draft.documents).length > 0;
export function useChapter(
  projectId: string,
  initial: ChapterSnapshot,
  onChanged: (snapshot: ChapterSnapshot) => void,
) {
  const [snapshot, setSnapshot] = useState(initial);
  const [draft, setDraft] = useState<ChapterDraft>(empty);
  const [state, setState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const [recoveryAvailable, setRecoveryAvailable] = useState(true);
  const current = useRef(initial);
  const drafts = useRef(draft);
  const failed = useRef(false);
  const reviewing = useRef(false);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const changed = useRef(onChanged);
  changed.current = onChanged;
  const persist = useCallback(() => {
    setRecoveryAvailable(
      storeChapterDraft(
        projectId,
        initial.chapter.id,
        current.current.globalRevision,
        drafts.current,
      ),
    );
  }, [projectId, initial.chapter.id]);
  const run = useCallback(
    (command: StoryCommand, submitted?: ChapterDraft): Promise<SubmitOutcome> => {
      if (reviewing.current) return Promise.resolve({ kind: "failed" });
      if (pending.current) return pending.current;
      if (timer.current) clearTimeout(timer.current);
      setState("saving");
      setError(null);
      const request = applyStory(projectId, current.current.globalRevision, command)
        .then((next): SubmitOutcome => {
          current.current = next;
          setSnapshot(next);
          changed.current(next);
          failed.current = false;
          if (submitted) {
            const remaining = { ...drafts.current.documents };
            for (const [area, content] of Object.entries(submitted.documents)) {
              if (JSON.stringify(remaining[area as DocumentArea]) === JSON.stringify(content))
                delete remaining[area as DocumentArea];
            }
            drafts.current = {
              title: drafts.current.title === submitted.title ? undefined : drafts.current.title,
              documents: remaining,
            };
            setDraft(drafts.current);
            persist();
          }
          setState(dirty(drafts.current) ? "dirty" : "saved");
          return { kind: dirty(drafts.current) ? "committed-stale" : "committed" };
        })
        .catch((reason: unknown): SubmitOutcome => {
          failed.current = true;
          setState("failed");
          setError(reason instanceof Error ? reason.message : "Writing could not be saved.");
          persist();
          return { kind: "failed" };
        })
        .finally(() => {
          pending.current = null;
        });
      pending.current = request;
      return request;
    },
    [projectId, persist],
  );
  const save = useCallback((): Promise<SubmitOutcome> => {
    if (reviewing.current) return Promise.resolve({ kind: "failed" });
    if (pending.current) return pending.current;
    if (!dirty(drafts.current)) return Promise.resolve({ kind: "no-op" });
    const submitted = drafts.current;
    return run(
      {
        kind: "save",
        chapterId: initial.chapter.id,
        title: submitted.title ?? null,
        documents: Object.entries(submitted.documents).map(([area, content]) => ({
          area: area as DocumentArea,
          content: content!,
          schemaVersion: 1,
        })),
      },
      submitted,
    );
  }, [initial.chapter.id, run]);
  const flush = useCallback(async (): Promise<SubmitOutcome> => {
    if (timer.current) clearTimeout(timer.current);
    let result = await save();
    // Navigation and native close drain text typed while an earlier write was pending.
    while (result.kind === "committed-stale") result = await save();
    return result;
  }, [save]);
  useEffect(() => {
    if (state !== "dirty" || failed.current) return;
    timer.current = setTimeout(() => void save(), 350);
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, [draft, state, save]);
  const change = useCallback(
    (next: ChapterDraft) => {
      drafts.current = next;
      setDraft(next);
      persist();
      // A failed save must not silently rebase or retry while the author keeps typing.
      setState(
        failed.current ? "failed" : pending.current ? "saving" : dirty(next) ? "dirty" : "saved",
      );
    },
    [persist],
  );
  function changeTitle(title: string) {
    change({ ...drafts.current, title });
  }
  function changeDocument(area: DocumentArea, content: JSONContent) {
    change({ ...drafts.current, documents: { ...drafts.current.documents, [area]: content } });
  }
  async function reloadSaved() {
    if (pending.current || reviewing.current) return undefined;
    reviewing.current = true;
    if (timer.current) clearTimeout(timer.current);
    setState("saving");
    try {
      const next = await readChapter(projectId, initial.chapter.id);
      const held = dirty(drafts.current)
        ? {
            version: 1 as const,
            revision: current.current.globalRevision,
            savedAt: new Date().toISOString(),
            draft: drafts.current,
          }
        : null;
      // Explicit comparison flow, never an automatic retry against a newer revision.
      current.current = next;
      setSnapshot(next);
      changed.current(next);
      drafts.current = empty();
      setDraft(empty());
      failed.current = false;
      setState("saved");
      setError(null);
      return held;
    } catch (reason) {
      failed.current = true;
      setState("failed");
      setError(reason instanceof Error ? reason.message : "The saved Chapter could not be loaded.");
      return undefined;
    } finally {
      reviewing.current = false;
    }
  }
  return {
    snapshot,
    draft,
    state,
    error,
    recoveryAvailable,
    flush,
    run,
    changeTitle,
    changeDocument,
    restoreDraft: change,
    reloadSaved,
  };
}
