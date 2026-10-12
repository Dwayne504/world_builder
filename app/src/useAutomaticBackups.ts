import { useCallback, useEffect, useRef, useState } from "react";
import { getAutomaticBackupStatus, setAutomaticBackupsEnabled } from "./api";
import type { AutomaticBackupStatus } from "./automaticBackupTypes";

const STATUS_POLL_MS = 30_000;

/** Observes the native scheduler. No frontend timer ever starts a backup. */
export function useAutomaticBackups(projectId: string) {
  const [status, setStatus] = useState<AutomaticBackupStatus | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [changing, setChanging] = useState(false);
  const changingRef = useRef(false);
  const mounted = useRef(false);
  const generation = useRef(0);
  const pending = useRef<Promise<void> | null>(null);

  const refresh = useCallback(
    (force = false): Promise<void> => {
      if (changingRef.current && !force) return Promise.resolve();
      if (pending.current && !force) return pending.current;
      const requestGeneration = ++generation.current;
      const request = getAutomaticBackupStatus(projectId)
        .then((next) => {
          if (!mounted.current || requestGeneration !== generation.current) return;
          setStatus(next);
          setReadError(null);
        })
        .catch((reason: unknown) => {
          if (!mounted.current || requestGeneration !== generation.current) return;
          setReadError(
            reason instanceof Error ? reason.message : "Backup status could not be loaded.",
          );
        })
        .finally(() => {
          if (pending.current === request) pending.current = null;
        });
      pending.current = request;
      return request;
    },
    [projectId],
  );

  useEffect(() => {
    mounted.current = true;
    setStatus(null);
    setReadError(null);
    setActionError(null);
    void refresh(true);
    const onFocus = () => {
      void refresh();
    };
    const interval = setInterval(() => {
      void refresh();
    }, STATUS_POLL_MS);
    window.addEventListener("focus", onFocus);
    return () => {
      mounted.current = false;
      clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh]);

  const setEnabled = useCallback(
    async (enabled: boolean) => {
      if (changingRef.current) return;
      changingRef.current = true;
      const operationGeneration = ++generation.current;
      setChanging(true);
      setActionError(null);
      try {
        const preferences = await setAutomaticBackupsEnabled(enabled);
        if (!mounted.current || operationGeneration !== generation.current) return;
        setStatus((previous) =>
          previous ? { ...previous, enabled: preferences.automaticBackupsEnabled } : previous,
        );
        await refresh(true);
      } catch (reason) {
        if (mounted.current && operationGeneration === generation.current)
          setActionError(
            reason instanceof Error ? reason.message : "Backup preference could not be saved.",
          );
      } finally {
        changingRef.current = false;
        if (mounted.current) setChanging(false);
      }
    },
    [refresh],
  );

  return { status, readError, actionError, changing, refresh, setEnabled };
}
