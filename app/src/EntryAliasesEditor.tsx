import { useEffect, useState } from "react";
import { applyAlias, readAliases } from "./api";
import type { AliasCommand, EntryAliases } from "./searchTypes";
import type { useMutationCoordinator } from "./useMutationCoordinator";

export function EntryAliasesEditor({
  projectId,
  entryId,
  disabled,
  mutations,
  getRevision,
  onRevision,
  onDraftChange,
}: {
  projectId: string;
  entryId: string;
  disabled: boolean;
  mutations: ReturnType<typeof useMutationCoordinator>;
  getRevision: () => number;
  onRevision: (revision: number) => void;
  onDraftChange: (dirty: boolean) => void;
}) {
  const [snapshot, setSnapshot] = useState<EntryAliases | null>(null);
  const [draft, setDraft] = useState("");
  const [removeId, setRemoveId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let current = true;
    void readAliases(projectId, entryId)
      .then((next) => {
        if (current) {
          setSnapshot(next);
          onRevision(next.globalRevision);
          setError(null);
        }
      })
      .catch((reason: unknown) => {
        if (current)
          setError(reason instanceof Error ? reason.message : "Aliases could not be loaded.");
      });
    return () => {
      current = false;
    };
  }, [projectId, entryId, onRevision, attempt]);
  function changeDraft(value: string) {
    setDraft(value);
    onDraftChange(!!value || !!removeId);
  }
  function confirmRemoval(id: string | null) {
    setRemoveId(id);
    onDraftChange(!!draft || !!id);
  }
  async function save(command: AliasCommand) {
    const outcome = await mutations.run(
      () => applyAlias(projectId, entryId, getRevision(), command),
      (next) => {
        setSnapshot(next);
        onRevision(next.globalRevision);
        setError(null);
        if (command.kind === "add") {
          setDraft("");
          onDraftChange(!!removeId);
        } else {
          setRemoveId(null);
          onDraftChange(!!draft);
        }
      },
    );
    if (outcome.kind === "failed") setError(outcome.errorMessage);
  }
  return (
    <section className="entry-aliases" aria-label="Entry aliases">
      <h3>Other names</h3>
      <p className="field-note">
        Aliases help you find this Entry. They leave its name and your writing unchanged.
      </p>
      {error && (
        <div role="alert">
          <p>{error}</p>
          <button disabled={disabled} onClick={() => setAttempt((n) => n + 1)}>
            Reload aliases
          </button>
        </div>
      )}
      <fieldset disabled={disabled || !snapshot}>
        <ul className="alias-list">
          {snapshot?.aliases.map((alias) => (
            <li key={alias.id}>
              <span>{alias.text}</span>
              {removeId === alias.id ? (
                <span className="row">
                  <span>Remove this alias?</span>
                  <button onClick={() => void save({ kind: "delete", aliasId: alias.id })}>
                    Remove alias
                  </button>
                  <button onClick={() => confirmRemoval(null)}>Keep alias</button>
                </span>
              ) : (
                <button
                  className="quiet-button"
                  aria-label={`Remove alias ${alias.text}`}
                  onClick={() => confirmRemoval(alias.id)}
                >
                  Remove
                </button>
              )}
            </li>
          ))}
        </ul>
        <label>
          New alias
          <input
            value={draft}
            maxLength={200}
            onChange={(event) => changeDraft(event.currentTarget.value)}
          />
        </label>
        <div className="row">
          <button
            disabled={!draft.trim() || !!removeId}
            onClick={() => void save({ kind: "add", text: draft })}
          >
            Add alias
          </button>
          {draft && (
            <button className="quiet-button" onClick={() => changeDraft("")}>
              Cancel alias
            </button>
          )}
        </div>
      </fieldset>
    </section>
  );
}
