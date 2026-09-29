import { FieldSuggestions } from "./FieldSuggestions";
import { fieldLabel } from "./fieldLabels";
import { useCallback, useEffect, useId, useState } from "react";
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
  number: "Number (optional unit)",
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
  const unitId = useId();
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
  const numeric = field.definition.kind === "number";
  const input = (
    <input
      aria-label={label}
      aria-describedby={field.definition.unit ? unitId : undefined}
      disabled={disabled}
      value={String(draft)}
      inputMode={numeric ? "decimal" : "text"}
      style={numeric ? { width: `${Math.max(2, String(draft).length + 1)}ch` } : undefined}
      onChange={(e) => onChange(e.target.value)}
      onBlur={onBlur}
    />
  );
  return numeric ? (
    <label className="number-value">
      {input}
      {field.definition.unit && (
        <span id={unitId} className="field-unit">
          {field.definition.unit}
        </span>
      )}
    </label>
  ) : (
    input
  );
}

export function EntryFieldsPanel({
  projectId,
  entry,
  disabled,
  onController,
  onRevision,
  getRevision,
  templateEpoch = 0,
}: {
  projectId: string;
  entry: Entry;
  disabled: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision?: () => number;
  templateEpoch?: number;
}) {
  const fields = useEntryFields(
    projectId,
    entry.id,
    `${entry.revision}:${templateEpoch}`,
    onRevision,
    getRevision,
  );
  const [manageOpen, setManageOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const { submit: submitValues } = fields;
  const [newName, setNewName] = useState("");
  const [newUnit, setNewUnit] = useState("");
  const [newKind, setNewKind] = useState<FieldKind>("short_text");
  const [newScope, setNewScope] = useState<FieldProvider["kind"]>("entry");
  const [newValue, setNewValue] = useState<FieldDraft>("");
  const [newOptions, setNewOptions] = useState("");
  const [manageId, setManageId] = useState("");
  const [renamed, setRenamed] = useState("");
  const [optionId, setOptionId] = useState("");
  const [optionLabel, setOptionLabel] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const formDirty = !!(
    newName ||
    String(newValue) ||
    newOptions ||
    newUnit ||
    renamed ||
    optionLabel
  );
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
    setCreateOpen(false);
    setNewName("");
    setNewUnit("");
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
          ...(newKind === "number" && newUnit.trim() ? { unit: newUnit.trim() } : {}),
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
        <div className="row">
          <button
            disabled={configDisabled || !!renamed || !!optionLabel}
            onClick={() => setCreateOpen(true)}
          >
            Add field
          </button>
          <button
            className="quiet-button"
            disabled={!!newName || !!String(newValue) || !!newOptions || !!newUnit}
            onClick={() => setManageOpen(true)}
          >
            Manage fields
          </button>
        </div>
      </div>
      {!fields.snapshot && !fields.error && <p role="status">Loading fields…</p>}
      {!manageOpen && !createOpen && (fields.error || formError) && (
        <p role="alert">{fields.error || formError}</p>
      )}
      {!manageOpen && !createOpen && fields.error && (
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
      {!createOpen && (newName || String(newValue) || newOptions || newUnit) && (
        <p className="field-note">
          You have an unfinished Field.{" "}
          <button className="quiet-button" onClick={() => setCreateOpen(true)}>
            Continue Field draft
          </button>
        </p>
      )}
      <div className="field-grid">
        {fields.snapshot?.fields.map((field) => (
          <div className="field-value" key={field.definition.id}>
            <label>{field.definition.name}</label>
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
              disabled={disabled || formDirty}
              onChange={(draft) => fields.change(field.definition.id, draft)}
              onBlur={() => void fields.submit()}
            />
            {(field.value !== null || fields.drafts[field.definition.id] !== undefined) && (
              <button
                className="quiet-button clear-field"
                aria-label={`Clear value: ${field.definition.name}`}
                disabled={disabled || formDirty}
                onClick={() => fields.change(field.definition.id, "")}
              >
                Clear
              </button>
            )}
          </div>
        ))}
      </div>
      <Dialog open={createOpen} title="Add field" onClose={() => setCreateOpen(false)}>
        <p>Create an optional Field for this Entry, or share it with its Category or Type.</p>
        {createOpen && (fields.error || formError) && (
          <p role="alert">{fields.error || formError}</p>
        )}
        {createOpen && fields.error && (
          <button disabled={busy} onClick={() => void fields.reload()}>
            Reload fields (keep drafts)
          </button>
        )}
        <fieldset
          disabled={configDisabled || !!renamed || !!optionLabel}
          className="inline-creator"
        >
          <label>
            Field name
            <input
              aria-label="new-field-name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
          </label>
          <label>
            Kind
            <select
              aria-label="new-field-kind"
              value={newKind}
              onChange={(e) => {
                setNewKind(e.target.value as FieldKind);
                setNewValue("");
                setNewOptions("");
                setNewUnit("");
              }}
            >
              {Object.entries(kinds).map(([kind, label]) => (
                <option key={kind} value={kind}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          {newKind === "number" && (
            <label>
              Unit (optional)
              <input
                aria-label="new-field-unit"
                placeholder="tons, km, years, gold crowns..."
                value={newUnit}
                onChange={(e) => setNewUnit(e.target.value)}
              />
            </label>
          )}
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
          <FieldSuggestions
            definitions={fields.snapshot?.definitions ?? []}
            name={newName}
            disabled={!!String(newValue) || !!newOptions}
            onReuse={(d) =>
              void configure(
                { kind: "bind", fieldId: d.id, provider: provider(newScope) },
                cancelNew,
              )
            }
          />
          {!!String(newValue) && (
            <p className="field-note">
              To reuse an existing Field, clear the initial value first, then fill it in on this
              Entry.
            </p>
          )}
          <div className="row">
            <button
              disabled={!newName.trim() || (newScope === "type" && !entry.typeId)}
              onClick={() => void create()}
            >
              Create field
            </button>
            <button onClick={cancelNew}>Cancel new field</button>
          </div>
        </fieldset>
      </Dialog>
      <Dialog open={manageOpen} title="Manage fields" onClose={() => setManageOpen(false)}>
        <p>
          Definition changes affect every Entry using that field. Detaching or retiring preserves
          existing values. To combine duplicates, open Categories → Combine duplicate fields.
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
          {[true, false].map((onEntry) => (
            <optgroup
              key={String(onEntry)}
              label={onEntry ? "Fields on this Entry" : "Other project fields"}
            >
              {fields.snapshot?.definitions
                .filter(
                  (d) => fields.snapshot!.fields.some((f) => f.definition.id === d.id) === onEntry,
                )
                .map((d) => (
                  <option key={d.id} value={d.id}>
                    {fieldLabel(d, fields.snapshot!.definitions)}
                  </option>
                ))}
            </optgroup>
          ))}
        </select>
        {selected && (
          <fieldset disabled={configDisabled || !!newName || !!String(newValue) || !!newOptions}>
            <legend>Shared definition: {selected.name}</legend>
            {selected.unit && <p>Unit: {selected.unit}. Unit conversion is not available yet.</p>}
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
            <p className="field-note">
              Removing a Field from new use retires it across this Project. Filled-in values stay
              visible and editable; you can restore the Field here.
            </p>
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
                {selected.retired ? "Restore field definition" : "Remove field from new use"}
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
