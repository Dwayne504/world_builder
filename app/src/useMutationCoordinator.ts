import { useCallback, useRef, useState } from "react";
import type { SaveState } from "./types";

export type MutationOutcome<T> =
  | { kind: "committed"; value: T }
  | { kind: "failed"; errorMessage: string }
  | { kind: "already-pending" };

export function useMutationCoordinator() {
  const [state, setState] = useState<SaveState>("saved");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const stateRef = useRef<SaveState>("saved");
  const errorRef = useRef<string | null>(null);
  const pendingRef = useRef<Promise<boolean> | null>(null);

  const run = useCallback(
    <T>(
      operation: () => Promise<T>,
      onCommitted?: (value: T) => void,
    ): Promise<MutationOutcome<T>> => {
      if (pendingRef.current) {
        return Promise.resolve({ kind: "already-pending" });
      }
      stateRef.current = "saving";
      setState("saving");
      setErrorMessage(null);
      errorRef.current = null;
      let resolvePending!: (successful: boolean) => void;
      pendingRef.current = new Promise((resolve) => {
        resolvePending = resolve;
      });
      return operation()
        .then((value): MutationOutcome<T> => {
          onCommitted?.(value);
          stateRef.current = "saved";
          setState("saved");
          resolvePending(true);
          return { kind: "committed", value };
        })
        .catch((error: unknown): MutationOutcome<T> => {
          const message = error instanceof Error ? error.message : "Structural save failed.";
          stateRef.current = "failed";
          setState("failed");
          errorRef.current = message;
          setErrorMessage(message);
          resolvePending(false);
          return { kind: "failed", errorMessage: message };
        })
        .finally(() => {
          pendingRef.current = null;
        });
    },
    [],
  );

  const waitForPending = useCallback(
    (): Promise<boolean> => pendingRef.current ?? Promise.resolve(stateRef.current !== "failed"),
    [],
  );

  const currentError = useCallback(() => errorRef.current, []);
  const isPending = useCallback(() => pendingRef.current !== null, []);

  return { state, errorMessage, run, waitForPending, currentError, isPending };
}
