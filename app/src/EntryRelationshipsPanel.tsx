import { useCallback, useEffect, useRef, useState } from "react";
import { Dialog } from "./Dialog";
import type { FieldsController } from "./EntryFieldsPanel";
import type {
  Category,
  Relationship,
  RelationshipCommand,
  RelationshipDefinition,
  RelationshipDraft,
} from "./types";
import type { SubmitOutcome } from "./useProjectRename";
import { useEntryRelationships } from "./useEntryRelationships";

const emptyDefinition: RelationshipDraft = {
  name: "",
  forwardLabel: "",
  inverseLabel: "",
  directed: true,
  expectedTargetsPerSource: null,
  expectedSourcesPerTarget: null,
};

export function EntryRelationshipsPanel({
  projectId,
  entryId,
  categories,
  disabled,
  onController,
  onRevision,
  getRevision,
  onNavigate,
}: {
  projectId: string;
  entryId: string;
  categories: Category[];
  disabled: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision: () => number;
  onNavigate: (id: string) => void;
}) {
  const data = useEntryRelationships(projectId, entryId, onRevision, getRevision);
  const { command, wait } = data;
  const [linkOpen, setLinkOpen] = useState(false);
  const [manageOpen, setManageOpen] = useState(false);
  const [definitionId, setDefinitionId] = useState("");
  const [perspective, setPerspective] = useState<"source" | "target">("source");
  const [target, setTarget] = useState("");
  const [createTarget, setCreateTarget] = useState(false);
  const [targetName, setTargetName] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [note, setNote] = useState("");
  const [replace, setReplace] = useState<string[]>([]);
  const [editId, setEditId] = useState("");
  const [draft, setDraft] = useState<RelationshipDraft>(emptyDefinition);
  const [definitionDirty, setDefinitionDirty] = useState(false);
  const [noteDraft, setNoteDraft] = useState<{ id: string; value: string } | null>(null);
  const noteRef = useRef(noteDraft);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const linkDirty = !!(definitionId || target || createTarget || note || replace.length);
  const formDirty = linkDirty || definitionDirty;
  const busy = disabled || data.state === "saving";
  const definition = data.snapshot?.definitions.find((d) => d.id === definitionId);
  const submitNote = useCallback(async (): Promise<SubmitOutcome> => {
    if (timer.current) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    const pending = await wait();
    if (pending.kind === "failed") return pending;
    const submitted = noteRef.current;
    if (!submitted) return { kind: "no-op" };
    const result = await command({ kind: "set_note", id: submitted.id, note: submitted.value });
    if (result.kind === "committed") {
      if (noteRef.current === submitted) {
        noteRef.current = null;
        setNoteDraft(null);
      } else return { kind: "committed-stale" };
    }
    return result;
  }, [command, wait]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  const submit = useCallback(async (): Promise<SubmitOutcome> => {
    // Close waits for an acknowledged operation before deciding whether a
    // remaining form needs attention; an in-flight write cannot be discarded.
    const pending = await wait();
    if (pending.kind === "failed") return pending;
    if (pending.kind === "committed" && formDirty) return pending;
    if (formDirty) {
      setFormError("Finish or cancel the relationship form before leaving.");
      return { kind: "failed" };
    }
    return submitNote();
  }, [formDirty, submitNote, wait]);
  const state =
    data.state === "saving" || data.state === "failed"
      ? data.state
      : formDirty || noteDraft
        ? "dirty"
        : "saved";
  useEffect(
    () => onController({ state, submit, canSubmit: !formDirty }),
    [state, submit, formDirty, onController],
  );

  function resetLink() {
    setDefinitionId("");
    setTarget("");
    setCreateTarget(false);
    setTargetName("");
    setCategoryId("");
    setNote("");
    setReplace([]);
    setFormError(null);
    setLinkOpen(false);
  }
  function resetDefinition() {
    setEditId("");
    setDraft(emptyDefinition);
    setDefinitionDirty(false);
    setFormError(null);
  }
  async function run(operation: RelationshipCommand, committed?: () => void) {
    setFormError(null);
    if ((await command(operation)).kind === "committed") committed?.();
  }
  function updateDraft(update: Partial<RelationshipDraft>) {
    setDraft({ ...draft, ...update });
    setDefinitionDirty(true);
  }
  function editDefinition(d: RelationshipDefinition) {
    setEditId(d.id);
    setDraft(d);
    setDefinitionDirty(false);
  }
  function describe(r: Relationship) {
    const d = data.snapshot?.definitions.find((d) => d.id === r.definitionId);
    const fromSource = r.source.id === entryId;
    return {
      label: fromSource ? d?.forwardLabel : d?.inverseLabel,
      other: fromSource ? r.target : r.source,
      definition: d,
    };
  }
  const replacements =
    data.snapshot?.relationships.filter(
      (r) =>
        !r.ended &&
        r.workspaceState === "active" &&
        r.definitionId === definitionId &&
        (!definition?.directed ||
          (perspective === "source" ? r.source.id === entryId : r.target.id === entryId)),
    ) ?? [];
  const configDisabled = busy || !!noteDraft || !data.snapshot;
  const current =
    data.snapshot?.relationships.filter((r) => !r.ended && r.workspaceState === "active") ?? [];
  const history =
    data.snapshot?.relationships.filter((r) => r.ended || r.workspaceState !== "active") ?? [];
  function connection(r: Relationship) {
    const view = describe(r);
    return (
      <li key={r.id} className="relationship-card">
        <div className="relationship-heading">
          <span>{view.label}</span>
          {view.other.id ? (
            <button
              className="relationship-target"
              disabled={busy}
              onClick={() => onNavigate(view.other.id!)}
            >
              {view.other.label}
            </button>
          ) : (
            <span>{view.other.label}</span>
          )}
          {view.other.workspaceState !== "active" && <small>{view.other.workspaceState}</small>}
          <small>
            {view.definition?.name}
            {view.definition?.retired ? " · retired definition" : ""}
            {r.ended ? " · ended" : ""}
            {r.workspaceState !== "active" ? ` · ${r.workspaceState}` : ""}
          </small>
        </div>
        {r.warnings.map((warning) => (
          <p key={warning} className="relationship-warning" role="status">
            {warning}
          </p>
        ))}
        <details className="relationship-details">
          <summary>Note and actions{r.note ? " · has note" : ""}</summary>
          <label>
            Relationship note
            <textarea
              aria-label={`Note: ${view.label} ${view.other.label}`}
              // Autosave must not disable the focused editor. Newer text stays
              // in noteRef until its own acknowledgement; other actions still wait.
              disabled={disabled || formDirty || (!!noteDraft && noteDraft.id !== r.id)}
              value={noteDraft?.id === r.id ? noteDraft.value : r.note}
              onChange={(e) => {
                const next = { id: r.id, value: e.target.value };
                noteRef.current = next;
                setNoteDraft(next);
                if (timer.current) clearTimeout(timer.current);
                timer.current = setTimeout(() => void submitNote(), 500);
              }}
            />
          </label>
          <div className="row">
            <button disabled={busy || noteDraft?.id !== r.id} onClick={() => void submitNote()}>
              Save note
            </button>
            <button
              disabled={configDisabled || r.workspaceState !== "active"}
              onClick={() => void run({ kind: "set_ended", id: r.id, ended: !r.ended })}
            >
              {r.ended ? "Restore relationship" : "End relationship"}
            </button>
          </div>
        </details>
      </li>
    );
  }
  return (
    <section className="relationships-panel">
      <div className="section-heading">
        <div>
          <p className="eyebrow">CONNECTIONS</p>
          <h3>Relationships</h3>
        </div>
        <div className="row">
          <button disabled={configDisabled || formDirty} onClick={() => setLinkOpen(true)}>
            Add relationship
          </button>
          <button
            className="quiet-button"
            disabled={configDisabled || linkDirty}
            onClick={() => setManageOpen(true)}
          >
            Manage relationships
          </button>
        </div>
      </div>
      {data.error && (
        <div className="error-banner" role="alert">
          {data.error} Your drafts are kept.{" "}
          <button disabled={busy} onClick={() => void data.reload()}>
            Reload relationships
          </button>
        </div>
      )}
      {formError && <p role="alert">{formError}</p>}
      {!data.snapshot ? (
        <p className="muted">Loading relationships…</p>
      ) : (
        !current.length && (
          <p className="muted">
            Connect this Entry to the people, places, and objects in your world.
          </p>
        )
      )}
      <ul className="relationship-list">{current.map(connection)}</ul>
      {!!history.length && (
        <details className="disclosure">
          <summary>Past and inactive relationships ({history.length})</summary>
          <ul className="relationship-list">{history.map(connection)}</ul>
        </details>
      )}
      <Dialog open={linkOpen} title="Add relationship" onClose={() => setLinkOpen(false)}>
        <p className="muted">One connection, visible from both Entries.</p>
        <label>
          Relationship definition
          <select
            aria-label="Relationship definition"
            disabled={busy}
            value={definitionId}
            onChange={(e) => {
              setDefinitionId(e.target.value);
              setReplace([]);
              setPerspective("source");
            }}
          >
            <option value="">Choose a definition</option>
            {data.snapshot?.definitions
              .filter((d) => !d.retired)
              .map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
          </select>
        </label>
        {!data.snapshot?.definitions.some((d) => !d.retired) && (
          <button
            disabled={busy}
            onClick={() => {
              setLinkOpen(false);
              setManageOpen(true);
            }}
          >
            Create a relationship definition
          </button>
        )}
        {definition && (
          <>
            <label>
              This Entry…
              <select
                aria-label="This Entry…"
                disabled={busy}
                value={perspective}
                onChange={(e) => {
                  setPerspective(e.target.value as "source" | "target");
                  setReplace([]);
                }}
              >
                <option value="source">{definition.forwardLabel}</option>
                {definition.directed && <option value="target">{definition.inverseLabel}</option>}
              </select>
            </label>
            <label>
              Connect to
              <select
                aria-label="Connect to"
                disabled={busy}
                value={createTarget ? "new" : "existing"}
                onChange={(e) => setCreateTarget(e.target.value === "new")}
              >
                <option value="existing">An existing Entry</option>
                <option value="new">Create an Entry here</option>
              </select>
            </label>
            {createTarget ? (
              <>
                <label>
                  New Entry name (optional)
                  <input
                    disabled={busy}
                    value={targetName}
                    onChange={(e) => setTargetName(e.target.value)}
                  />
                </label>
                <label>
                  New Entry Category
                  <select
                    aria-label="New Entry Category"
                    disabled={busy}
                    value={categoryId}
                    onChange={(e) => setCategoryId(e.target.value)}
                  >
                    <option value="">Uncategorized</option>
                    {categories
                      .filter((c) => !c.isUncategorized)
                      .map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.name}
                        </option>
                      ))}
                  </select>
                </label>
              </>
            ) : (
              <label>
                Other Entry
                <select
                  aria-label="Other Entry"
                  disabled={busy}
                  value={target}
                  onChange={(e) => setTarget(e.target.value)}
                >
                  <option value="">Choose an Entry</option>
                  {data.snapshot?.entries.map((e) => (
                    <option key={e.id} value={e.id}>
                      {e.label} · {e.categoryName}
                      {e.id === entryId ? " (this Entry)" : ""}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <label>
              Note (optional)
              <textarea disabled={busy} value={note} onChange={(e) => setNote(e.target.value)} />
            </label>
            {!!replacements.length && (
              <details className="disclosure">
                <summary>Replace an existing relationship…</summary>
                <p>
                  Selected relationships will be ended and kept in history. Leave them unchecked to
                  keep them current.
                </p>
                {replacements.map((r) => (
                  <label className="checkbox-row" key={r.id}>
                    <input
                      type="checkbox"
                      disabled={busy}
                      checked={replace.includes(r.id)}
                      onChange={(e) =>
                        setReplace(
                          e.target.checked
                            ? [...replace, r.id]
                            : replace.filter((id) => id !== r.id),
                        )
                      }
                    />
                    End {describe(r).label} {describe(r).other.label}
                  </label>
                ))}
              </details>
            )}
            <p className="muted">
              Conflicting relationships are kept and flagged. Nothing is replaced automatically.
            </p>
          </>
        )}
        {data.error && <p role="alert">{data.error}</p>}
        <div className="row">
          <button
            disabled={busy || !definition || (!createTarget && !target)}
            onClick={() =>
              void run(
                {
                  kind: "connect",
                  definitionId,
                  perspective,
                  other: createTarget
                    ? { kind: "create", name: targetName || null, categoryId: categoryId || null }
                    : { kind: "existing", id: target },
                  note,
                  replace,
                },
                resetLink,
              )
            }
          >
            {replace.length ? "End selected and connect" : "Create relationship"}
          </button>
          <button disabled={busy} onClick={resetLink}>
            Cancel relationship
          </button>
        </div>
      </Dialog>
      <Dialog
        open={manageOpen}
        title="Relationship definitions"
        onClose={() => setManageOpen(false)}
      >
        <p className="muted">Define the meaning once, then reuse it throughout this Project.</p>
        <label>
          Definition to edit
          <select
            aria-label="Definition to edit"
            disabled={busy || definitionDirty}
            value={editId}
            onChange={(e) => {
              const d = data.snapshot?.definitions.find((d) => d.id === e.target.value);
              if (d) editDefinition(d);
              else resetDefinition();
            }}
          >
            <option value="">New definition</option>
            {data.snapshot?.definitions.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
                {d.retired ? " (retired)" : ""}
              </option>
            ))}
          </select>
        </label>
        <label>
          Definition name
          <input
            disabled={busy}
            placeholder="Ownership"
            value={draft.name}
            onChange={(e) => updateDraft({ name: e.target.value })}
          />
        </label>
        <label>
          Direction
          <select
            aria-label="Direction"
            disabled={busy || !!editId}
            value={draft.directed ? "directed" : "symmetric"}
            onChange={(e) => updateDraft({ directed: e.target.value === "directed" })}
          >
            <option value="directed">Different meaning on each side</option>
            <option value="symmetric">Same meaning on both sides</option>
          </select>
        </label>
        <label>
          {draft.directed ? "Forward label" : "Relationship label"}
          <input
            disabled={busy}
            placeholder={draft.directed ? "owns" : "is allied with"}
            value={draft.forwardLabel}
            onChange={(e) => updateDraft({ forwardLabel: e.target.value })}
          />
        </label>
        {draft.directed && (
          <label>
            Inverse label
            <input
              disabled={busy}
              placeholder="is owned by"
              value={draft.inverseLabel}
              onChange={(e) => updateDraft({ inverseLabel: e.target.value })}
            />
          </label>
        )}
        <details className="disclosure">
          <summary>Optional expectations</summary>
          <p>
            These show warnings when exceeded. They never prevent you from recording a relationship.
          </p>
          <label>
            Expected maximum per {draft.directed ? "source" : "Entry"}
            <input
              disabled={busy}
              type="number"
              min="1"
              step="1"
              value={draft.expectedTargetsPerSource ?? ""}
              onChange={(e) =>
                updateDraft({
                  expectedTargetsPerSource: e.target.value ? Number(e.target.value) : null,
                })
              }
            />
          </label>
          {draft.directed && (
            <label>
              Expected maximum per target
              <input
                disabled={busy}
                type="number"
                min="1"
                step="1"
                value={draft.expectedSourcesPerTarget ?? ""}
                onChange={(e) =>
                  updateDraft({
                    expectedSourcesPerTarget: e.target.value ? Number(e.target.value) : null,
                  })
                }
              />
            </label>
          )}
        </details>
        {data.error && <p role="alert">{data.error}</p>}
        <div className="row">
          <button
            disabled={
              busy ||
              !draft.name.trim() ||
              !draft.forwardLabel.trim() ||
              (draft.directed && !draft.inverseLabel.trim())
            }
            onClick={() => {
              const validated = draft.directed
                ? draft
                : {
                    ...draft,
                    inverseLabel: draft.forwardLabel,
                    expectedSourcesPerTarget: draft.expectedTargetsPerSource,
                  };
              void run(
                editId
                  ? { kind: "update_definition", definitionId: editId, draft: validated }
                  : { kind: "create_definition", draft: validated },
                () => {
                  resetDefinition();
                  setManageOpen(false);
                },
              );
            }}
          >
            {editId ? "Save definition" : "Create definition"}
          </button>
          <button
            disabled={busy}
            onClick={() => {
              resetDefinition();
              setManageOpen(false);
            }}
          >
            Cancel definition
          </button>
          {editId && (
            <button
              disabled={busy || definitionDirty}
              onClick={() =>
                void run(
                  {
                    kind: "retire_definition",
                    definitionId: editId,
                    retired: !data.snapshot?.definitions.find((d) => d.id === editId)?.retired,
                  },
                  () => {
                    resetDefinition();
                    setManageOpen(false);
                  },
                )
              }
            >
              {data.snapshot?.definitions.find((d) => d.id === editId)?.retired
                ? "Restore definition"
                : "Retire definition"}
            </button>
          )}
        </div>
      </Dialog>
      {!linkOpen && linkDirty && (
        <button onClick={() => setLinkOpen(true)}>Continue relationship draft</button>
      )}
      {!manageOpen && definitionDirty && (
        <button onClick={() => setManageOpen(true)}>Continue definition draft</button>
      )}
    </section>
  );
}
