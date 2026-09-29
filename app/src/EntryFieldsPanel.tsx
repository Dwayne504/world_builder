import { useCallback, useEffect, useState } from "react";
import type { Entry, EntryField, FieldCommand, FieldKind, FieldProvider, SaveState } from "./types";
import type { SubmitOutcome } from "./useProjectRename";
import { parseFieldDraft, useEntryFields, valueDraft, type FieldDraft } from "./useEntryFields";
import { Dialog } from "./Dialog";

export interface FieldsController {
  state: SaveState;
  submit: () => Promise<SubmitOutcome>;
  canSubmit: boolean;
}
const kinds: Record<FieldKind, string> = {
  short_text: "Short Text",
  number: "Number",
  boolean: "Boolean",
  choice: "Choice",
  multi_choice: "Multi-choice",
};

function ValueInput({
  field,
  draft,
  disabled,
  onChange,
  onBlur,
}: {
  field: EntryField;
  draft: FieldDraft;
  disabled: boolean;
  onChange: (draft: FieldDraft) => void;
  onBlur?: () => void;
}) {
  const label = `Value: ${field.definition.name}`;
  const selected = Array.isArray(draft) ? draft : draft ? [draft] : [];
  if (field.definition.kind === "boolean")
    return (
      <select
        aria-label={label}
        disabled={disabled}
        value={String(draft)}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
      >
        <option value="">Not filled in</option>
        <option value="true">Yes</option>
        <option value="false">No</option>
      </select>
    );
  if (field.definition.kind === "choice" || field.definition.kind === "multi_choice")
    return (
      <select
        aria-label={label}
        disabled={disabled}
        multiple={field.definition.kind === "multi_choice"}
        value={field.definition.kind === "multi_choice" ? selected : (selected[0] ?? "")}
        onChange={(e) =>
          onChange(
            Array.from(e.target.selectedOptions)
              .map((o) => o.value)
              .filter(Boolean),
          )
        }
        onBlur={onBlur}
      >
        {field.definition.kind === "choice" && <option value="">Not filled in</option>}
        {field.definition.options.map((o) => (
          <option key={o.id} value={o.id} disabled={o.retired && !selected.includes(o.id)}>
            {o.label}
            {o.retired ? " (retired)" : ""}
          </option>
        ))}
      </select>
    );
  return (
    <input
      aria-label={label}
      disabled={disabled}
      value={String(draft)}
      inputMode={field.definition.kind === "number" ? "decimal" : "text"}
      onChange={(e) => onChange(e.target.value)}
      onBlur={onBlur}
    />
  );
}

export function EntryFieldsPanel({
  projectId,
  entry,
  disabled,
  onController,
  onRevision,
  getRevision,
}: {
  projectId: string;
  entry: Entry;
  disabled: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision?: () => number;
}) {
  const fields = useEntryFields(projectId, entry.id, entry.revision, onRevision, getRevision);
  const [manageOpen, setManageOpen] = useState(false);
  const { submit: submitValues } = fields;
  const [newName, setNewName] = useState("");
  const [newKind, setNewKind] = useState<FieldKind>("short_text");
  const [newScope, setNewScope] = useState<FieldProvider["kind"]>("entry");
  const [newValue, setNewValue] = useState<FieldDraft>("");
  const [newOptions, setNewOptions] = useState("");
  const [manageId, setManageId] = useState("");
  const [renamed, setRenamed] = useState("");
  const [optionId, setOptionId] = useState("");
  const [optionLabel, setOptionLabel] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const formDirty = !!(newName || String(newValue) || newOptions || renamed || optionLabel);
  const combinedState =
    fields.state === "saving" || fields.state === "failed"
      ? fields.state
      : formDirty
        ? "dirty"
        : fields.state;
  const submit = useCallback((): Promise<SubmitOutcome> => {
    if (formDirty) {
      setFormError("Finish or cancel the field definition form before leaving.");
      return Promise.resolve({ kind: "failed" });
    }
    return submitValues();
  }, [formDirty, submitValues]);
  useEffect(
    () => onController({ state: combinedState, submit, canSubmit: !formDirty }),
    [combinedState, submit, formDirty, onController],
  );
  const busy = disabled || fields.state === "saving";
  const hasValueDraft = Object.keys(fields.drafts).length > 0;
  const configDisabled = busy || hasValueDraft || !fields.snapshot;
  const selected = fields.snapshot?.definitions.find((d) => d.id === manageId);
  const provider = (kind: FieldProvider["kind"]): FieldProvider => ({
    kind,
    id: kind === "entry" ? entry.id : kind === "category" ? entry.categoryId : entry.typeId!,
  });

  async function configure(command: FieldCommand, onSaved?: () => void) {
    setFormError(null);
    const result = await fields.command(command);
    if (result.kind === "committed") onSaved?.();
  }
  function cancelNew() {
    setNewName("");
    setNewValue("");
    setNewOptions("");
    setFormError(null);
  }
  async function create() {
    try {
      const value = parseFieldDraft(
        {
          definition: {
            id: "",
            name: newName,
            kind: newKind,
            retired: false,
            revision: 0,
            options: [],
            bindings: [],
          },
          available: true,
          value: null,
        },
        newValue,
      );
      await configure(
        {
          kind: "create",
          name: newName,
          fieldKind: newKind,
          provider: provider(newScope),
          options: newOptions
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean),
          value,
        },
        cancelNew,
      );
    } catch (err) {
      setFormError((err as Error).message);
    }
  }

  return (
    <section aria-label="Entry fields" className="fields-panel">
      <div className="section-heading">
        <div>
          <h3>Fields</h3>
          <p className="muted">Add only what helps tell this Entry's story.</p>
        </div>
        <button className="quiet-button" onClick={() => setManageOpen(true)}>
          Manage fields
        </button>
      </div>
      {!fields.snapshot && !fields.error && <p role="status">Loading fields…</p>}
      {!manageOpen && (fields.error || formError) && (
        <p role="alert">{fields.error || formError}</p>
      )}
      {!manageOpen && fields.error && (
        <button disabled={busy} onClick={() => void fields.reload()}>
          Reload fields (keep drafts)
        </button>
      )}
      {hasValueDraft && (
        <button disabled={busy} onClick={() => void fields.submit()}>
          Save field values
        </button>
      )}
      {fields.snapshot?.fields.length === 0 && (
        <p className="empty-state">No fields yet. A name and a value are enough to start.</p>
      )}
      {!manageOpen && (renamed || optionLabel) && (
        <p className="field-note">
          You have unfinished definition edits.{" "}
          <button className="quiet-button" onClick={() => setManageOpen(true)}>
            Continue definition edits
          </button>
        </p>
      )}
      {fields.snapshot?.fields.map((field) => (
        <div className="field-value" key={field.definition.id}>
          <label>
            {field.definition.name} <small>({kinds[field.definition.kind]})</small>
          </label>
          {!field.available && (
            <p className="field-note">
              {field.definition.retired
                ? "Retired field — authored value retained."
                : "Detached from the current templates — authored value retained."}
            </p>
          )}
          <ValueInput
            field={field}
            draft={fields.drafts[field.definition.id] ?? valueDraft(field.value)}
            disabled={busy || formDirty}
            onChange={(draft) => fields.change(field.definition.id, draft)}
            onBlur={() => void fields.submit()}
          />
          {(field.value !== null || fields.drafts[field.definition.id] !== undefined) && (
            <button
              className="quiet-button clear-field"
              aria-label={`Clear value: ${field.definition.name}`}
              disabled={busy || formDirty}
              onClick={() => fields.change(field.definition.id, "")}
            >
              Clear
            </button>
          )}
        </div>
      ))}
      <details className="disclosure add-field">
        <summary>
          Add a field{newName || String(newValue) || newOptions ? " · unfinished" : ""}
        </summary>
        <fieldset
          disabled={configDisabled || !!renamed || !!optionLabel}
          className="inline-creator"
        >
          <legend>Add a field</legend>
          <label>
            Field name
            <input
              aria-label="new-field-name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
          </label>
          <details className="field-options">
            <summary>
              Field options · {kinds[newKind]} ·{" "}
              {newScope === "entry"
                ? "This Entry"
                : newScope === "type"
                  ? "This Type"
                  : "This Category"}
            </summary>
            <label>
              Kind
              <select
                aria-label="new-field-kind"
                value={newKind}
                onChange={(e) => {
                  setNewKind(e.target.value as FieldKind);
                  setNewValue("");
                  setNewOptions("");
                }}
              >
                {Object.entries(kinds).map(([kind, label]) => (
                  <option key={kind} value={kind}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Make available to
              <select
                aria-label="new-field-scope"
                value={newScope}
                onChange={(e) => setNewScope(e.target.value as FieldProvider["kind"])}
              >
                <option value="entry">This Entry only</option>
                <option value="category">This Category</option>
                {entry.typeId && <option value="type">This Type</option>}
              </select>
            </label>
          </details>
          {newKind === "choice" || newKind === "multi_choice" ? (
            <label>
              Options (one per line)
              <textarea
                aria-label="new-field-options"
                value={newOptions}
                onChange={(e) => setNewOptions(e.target.value)}
              />
            </label>
          ) : (
            <label>
              Initial value (optional)
              {newKind === "boolean" ? (
                <select
                  aria-label="new-field-value"
                  value={String(newValue)}
                  onChange={(e) => setNewValue(e.target.value)}
                >
                  <option value="">Not filled in</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              ) : (
                <input
                  aria-label="new-field-value"
                  value={String(newValue)}
                  onChange={(e) => setNewValue(e.target.value)}
                />
              )}
            </label>
          )}
          <div className="row">
            <button
              disabled={!newName.trim() || (newScope === "type" && !entry.typeId)}
              onClick={() => void create()}
            >
              Add field
            </button>
            <button onClick={cancelNew}>Cancel new field</button>
          </div>
        </fieldset>
      </details>
      <Dialog open={manageOpen} title="Manage fields" onClose={() => setManageOpen(false)}>
        <p>
          Definition changes affect every Entry using that field. Detaching or retiring preserves
          existing values.
        </p>
        {manageOpen && (fields.error || formError) && (
          <p role="alert" className="error-banner">
            {fields.error || formError}
          </p>
        )}
        {manageOpen && fields.error && (
          <button disabled={busy} onClick={() => void fields.reload()}>
            Reload fields (keep drafts)
          </button>
        )}
        <select
          aria-label="field-definition"
          disabled={configDisabled || formDirty}
          value={manageId}
          onChange={(e) => {
            setManageId(e.target.value);
            setOptionId("");
          }}
        >
          <option value="">Choose a definition</option>
          {fields.snapshot?.definitions.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
              {d.retired ? " (retired)" : ""}
              {fields.snapshot!.definitions.filter((other) => other.name === d.name).length > 1
                ? ` · ${kinds[d.kind]} · ${d.id.slice(-8)}`
                : ""}
            </option>
          ))}
        </select>
        {selected && (
          <fieldset disabled={configDisabled || !!newName || !!String(newValue) || !!newOptions}>
            <legend>Shared definition: {selected.name}</legend>
            <label>
              New definition name
              <input
                aria-label="rename-field"
                value={renamed}
                onChange={(e) => setRenamed(e.target.value)}
              />
            </label>
            <button
              disabled={!renamed.trim() || !!optionLabel}
              onClick={() =>
                void configure({ kind: "rename", fieldId: selected.id, name: renamed }, () =>
                  setRenamed(""),
                )
              }
            >
              Rename definition
            </button>
            <button
              onClick={() => {
                setRenamed("");
                setOptionLabel("");
                setFormError(null);
              }}
            >
              Cancel definition edits
            </button>
            <div className="row">
              <button
                disabled={!!renamed || !!optionLabel}
                onClick={() =>
                  void configure({
                    kind: "set_retired",
                    fieldId: selected.id,
                    retired: !selected.retired,
                  })
                }
              >
                {selected.retired ? "Restore field definition" : "Retire field definition"}
              </button>
              {(["entry", "category", "type"] as const)
                .filter((kind) => kind !== "type" || entry.typeId)
                .map((kind) => (
                  <button
                    key={kind}
                    disabled={
                      selected.retired ||
                      !!renamed ||
                      !!optionLabel ||
                      selected.bindings.some(
                        (b) => b.provider.kind === kind && b.provider.id === provider(kind).id,
                      )
                    }
                    onClick={() =>
                      void configure({
                        kind: "bind",
                        fieldId: selected.id,
                        provider: provider(kind),
                      })
                    }
                  >
                    Make available to this{" "}
                    {kind === "entry" ? "Entry" : kind === "type" ? "Type" : "Category"}
                  </button>
                ))}
            </div>
            <ul>
              {selected.bindings.map((b) => (
                <li key={`${b.provider.kind}:${b.provider.id}`}>
                  {b.provider.kind}: {b.label}{" "}
                  <button
                    disabled={!!renamed || !!optionLabel}
                    onClick={() =>
                      void configure({ kind: "unbind", fieldId: selected.id, provider: b.provider })
                    }
                  >
                    Detach from {b.provider.kind}: {b.label}
                  </button>
                </li>
              ))}
            </ul>
            {(selected.kind === "choice" || selected.kind === "multi_choice") && (
              <>
                <label>
                  Choice option
                  <select
                    aria-label="choice-option"
                    disabled={!!optionLabel}
                    value={optionId}
                    onChange={(e) => setOptionId(e.target.value)}
                  >
                    <option value="">Add a new option</option>
                    {selected.options.map((o) => (
                      <option key={o.id} value={o.id}>
                        {o.label}
                        {o.retired ? " (retired)" : ""}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Option label
                  <input
                    aria-label="choice-option-label"
                    value={optionLabel}
                    onChange={(e) => setOptionLabel(e.target.value)}
                  />
                </label>
                <button
                  disabled={!optionLabel.trim() || !!renamed || (!optionId && selected.retired)}
                  onClick={() =>
                    void configure(
                      optionId
                        ? { kind: "rename_choice", optionId, label: optionLabel }
                        : { kind: "add_choice", fieldId: selected.id, label: optionLabel },
                      () => setOptionLabel(""),
                    )
                  }
                >
                  {optionId ? "Rename option" : "Add option"}
                </button>
                {optionId && (
                  <button
                    disabled={!!optionLabel || !!renamed}
                    onClick={() =>
                      void configure({
                        kind: "set_choice_retired",
                        optionId,
                        retired: !selected.options.find((o) => o.id === optionId)?.retired,
                      })
                    }
                  >
                    {selected.options.find((o) => o.id === optionId)?.retired
                      ? "Restore option"
                      : "Retire option"}
                  </button>
                )}
              </>
            )}
          </fieldset>
        )}
      </Dialog>
    </section>
  );
}
