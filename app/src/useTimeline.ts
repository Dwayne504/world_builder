import { useCallback, useEffect, useRef, useState } from "react";
import { applyTimeline, readTimeline } from "./api";
import {
  dateError,
  occurrenceDraft,
  type OccurrenceDraft,
  type TimelineCommand,
  type TimelineSnapshot,
  type WorldCalendar,
} from "./timelineTypes";
import type { SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";

export function useTimeline(projectId: string, onRevision: (revision: number) => void) {
  const [snapshot, setSnapshot] = useState<TimelineSnapshot | null>(null);
  const [draft, setDraft] = useState<OccurrenceDraft | null>(null);
  const [calendarDraft, setCalendarDraft] = useState<WorldCalendar | null>(null);
  const [state, setState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const saved = useRef(snapshot);
  const editing = useRef<{ id: string; draft: OccurrenceDraft } | null>(null);
  const calendar = useRef<WorldCalendar | null>(null);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const failed = useRef(false);
  const dirty = useRef(false);
  const changed = useRef(onRevision);
  changed.current = onRevision;
  const accept = useCallback((next: TimelineSnapshot) => {
    saved.current = next;
    setSnapshot(next);
    changed.current(next.globalRevision);
  }, []);
  const reload = useCallback(async () => {
    if (pending.current || dirty.current || calendar.current) return;
    try {
      accept(await readTimeline(projectId));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Timeline could not be loaded.");
    }
  }, [projectId, accept]);
  useEffect(() => {
    void reload();
  }, [reload]);
  const run = useCallback(
    (command: TimelineCommand): Promise<SubmitOutcome> => {
      if (pending.current) return pending.current;
      if (!saved.current) return Promise.resolve({ kind: "failed" });
      setState("saving");
      setError(null);
      const task = applyTimeline(projectId, saved.current.globalRevision, command)
        .then((next): SubmitOutcome => {
          accept(next);
          failed.current = false;
          if (command.kind === "save" && editing.current?.draft === command.draft) {
            dirty.current = false;
          }
          if (command.kind === "configure_calendar" && calendar.current === command.calendar) {
            calendar.current = null;
            setCalendarDraft(null);
          }
          const remaining = dirty.current || !!calendar.current;
          setState(remaining ? "dirty" : "saved");
          return { kind: remaining ? "committed-stale" : "committed" };
        })
        .catch((e: unknown): SubmitOutcome => {
          failed.current = true;
          setState("failed");
          setError(
            e instanceof Error
              ? e.message
              : "Timeline could not be saved. Your edits are still here.",
          );
          return { kind: "failed" };
        })
        .finally(() => {
          pending.current = null;
        });
      pending.current = task;
      return task;
    },
    [projectId, accept],
  );
  const save = useCallback((): Promise<SubmitOutcome> => {
    if (pending.current) return pending.current;
    if (calendar.current) return run({ kind: "configure_calendar", calendar: calendar.current });
    if (dirty.current && editing.current) {
      const problem = dateError(editing.current.draft.date, saved.current?.calendar ?? null);
      if (problem) {
        setError(problem);
        return Promise.resolve({ kind: "failed" });
      }
      return run({ kind: "save", ...editing.current });
    }
    return Promise.resolve({ kind: "no-op" });
  }, [run]);
  const flush = useCallback(async (): Promise<SubmitOutcome> => {
    let outcome = await save();
    while (outcome.kind === "committed-stale") outcome = await save();
    return outcome;
  }, [save]);
  useEffect(() => {
    if (
      state !== "dirty" ||
      failed.current ||
      calendarDraft ||
      !draft ||
      dateError(draft.date, snapshot?.calendar ?? null)
    )
      return;
    const timer = setTimeout(() => void save(), 500);
    return () => clearTimeout(timer);
  }, [state, draft, calendarDraft, snapshot?.calendar, save]);
  function edit(id: string | null) {
    if (pending.current || dirty.current || calendar.current) return;
    const item = saved.current?.occurrences.find((o) => o.id === id);
    editing.current = item ? { id: item.id, draft: occurrenceDraft(item) } : null;
    setDraft(editing.current?.draft ?? null);
  }
  function change(next: OccurrenceDraft) {
    if (!editing.current) return;
    editing.current = { id: editing.current.id, draft: next };
    dirty.current = true;
    setDraft(next);
    setState(failed.current ? "failed" : pending.current ? "saving" : "dirty");
  }
  function changeCalendar(next: WorldCalendar) {
    calendar.current = next;
    setCalendarDraft(next);
    setState(failed.current ? "failed" : pending.current ? "saving" : "dirty");
  }
  async function discard() {
    if (pending.current) return;
    dirty.current = false;
    failed.current = false;
    calendar.current = null;
    editing.current = null;
    setDraft(null);
    setCalendarDraft(null);
    setState("saved");
    await reload();
  }
  return {
    getSnapshot: () => saved.current,
    snapshot,
    draft,
    calendarDraft,
    state,
    error,
    edit,
    change,
    changeCalendar,
    discard,
    reload,
    run,
    flush,
  };
}
