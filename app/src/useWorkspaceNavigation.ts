import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { applyWorkspaceNavigation, readWorkspaceNavigation } from "./api";
import type { NavigationCommand, NavigationSnapshot } from "./workspaceNavigationTypes";

/** Navigation metadata has its own queue; it never changes authored save state. */
export function useWorkspaceNavigation(projectId: string) {
  const scope = useMemo(
    () => ({ projectId, active: false, generation: 0, tail: Promise.resolve(), pending: 0 }),
    [projectId],
  );
  const [snapshot, setSnapshot] = useState<NavigationSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const failed = useRef<{ command: NavigationCommand | null } | null>(null);
  const enqueue = useCallback(
    (command: NavigationCommand | null, retry = false) => {
      const generation = scope.generation;
      scope.pending += 1;
      setBusy(true);
      const current = () => scope.active && generation === scope.generation;
      const operation = scope.tail.then(async () => {
        if (!current()) return;
        try {
          const result = command
            ? await applyWorkspaceNavigation(scope.projectId, command)
            : await readWorkspaceNavigation(scope.projectId);
          if (!current()) return;
          setSnapshot(result);
          if (retry || failed.current?.command === null) {
            failed.current = null;
            setError(null);
          }
        } catch (reason) {
          if (!current()) return;
          // Passive refresh/visit failures must not erase the author's failed action.
          const explicitFailure =
            failed.current?.command && failed.current.command.kind !== "visit";
          if (!explicitFailure || (command && command.kind !== "visit")) {
            failed.current = { command };
            setError(
              reason instanceof Error ? reason.message : "Navigation shortcuts are unavailable.",
            );
          }
        } finally {
          if (current()) {
            scope.pending -= 1;
            setBusy(scope.pending > 0);
          }
        }
      });
      scope.tail = operation;
      return operation;
    },
    [scope],
  );
  useEffect(() => {
    scope.active = true;
    scope.generation += 1;
    scope.pending = 0;
    setSnapshot(null);
    setError(null);
    failed.current = null;
    void enqueue(null);
    return () => {
      scope.active = false;
    };
  }, [scope, enqueue]);
  const refresh = useCallback(() => enqueue(null), [enqueue]);
  const apply = useCallback((command: NavigationCommand) => enqueue(command), [enqueue]);
  const retry = useCallback(() => enqueue(failed.current?.command ?? null, true), [enqueue]);
  return { snapshot, busy, error, refresh, apply, retry };
}
