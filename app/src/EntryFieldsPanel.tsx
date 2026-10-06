import { FieldManagerTables } from "./FieldManagerTables";
import { DeleteEntryFieldDialog } from "./DeleteEntryFieldDialog";
import { FieldSuggestions } from "./FieldSuggestions";
import { ProjectionConfiguration } from "./ProjectionConfiguration";
import { ProjectionFieldValue } from "./ProjectionFieldValue";
import { useCallback, useEffect, useId, useState } from "react";
import type {
  Entry,
  EntryField,
  FieldCommand,
  FieldKind,
  FieldProjection,
  FieldProvider,
  SaveState,
} from "./types";
import type { SubmitOutcome } from "./useProjectRename";
import { parseFieldDraft, useEntryFields, valueDraft, type FieldDraft } from "./useEntryFields";
import { Dialog } from "./Dialog";
import { useDesktopCommands } from "./desktopMenuContext";
import { ManagerSearchSelect } from "./ManagerSearchSelect";

export interface FieldsController {
  state: SaveState;
  submit: () => Promise<SubmitOutcome>;
  canSubmit: boolean;
  waitForPending?: () => Promise<SubmitOutcome>;
}
const kinds: Record<FieldKind, string> = {
  short_text: "Short Text",
  number: "Number (optional unit)",
  boolean: "Boolean",
  choice: "Choice",
  multi_choice: "Multi-choice",
  relationship: "Relationship",
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
  onRecoveryBackup,
  refreshKey = 0,
  onCommitted,
  onPresentedRelationships,
  onNavigate,
  onEntriesChanged,
}: {
  projectId: string;
  entry: Entry;
  disabled: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision?: () => number;
  templateEpoch?: number;
  onRecoveryBackup?: (path: string) => void;
  refreshKey?: number;
  onCommitted?: (revision: number) => void;
  onPresentedRelationships?: (ids: string[]) => void;
  onNavigate?: (id: string) => void;
  onEntriesChanged?: () => void;
}) {
  const fields = useEntryFields(
    projectId,
    entry.id,
    `${entry.revision}:${templateEpoch}:${refreshKey}`,
    onRevision,
    getRevision,
    onCommitted,
  );
  const [manageOpen, setManageOpen] = useState(false);
  const [definitionOpen, setDefinitionOpen] = useState(false);
  const [showHidden, setShowHidden] = useState(false);
  const [deleteReview, setDeleteReview] = useState<{ field: EntryField; revision: number } | null>(
    null,
  );
  const [createOpen, setCreateOpen] = useState(false);
  const { submit: submitValues, waitForPending, command: commandFields } = fields;
  const editProjection = useCallback(
    async (command: FieldCommand): Promise<SubmitOutcome> => {
      const result = await commandFields(command);
      if (
        result.kind === "committed" &&
        command.kind === "edit_projection" &&
        command.other.kind === "create"
      ) {
        onEntriesChanged?.();
      }
      return result;
    },
    [commandFields, onEntriesChanged],
  );
  const [newName, setNewName] = useState("");
  const [newUnit, setNewUnit] = useState("");
  const [newKind, setNewKind] = useState<FieldKind>("short_text");
  const [projection, setProjection] = useState<FieldProjection>({
    relationshipDefinitionId: "",
    perspective: "source",
  });
  const [projectionDrafts, setProjectionDrafts] = useState<string[]>([]);
  const updateProjectionDirty = useCallback((id: string, dirty: boolean) => {
    setProjectionDrafts((current) =>
      dirty
        ? current.includes(id)
          ? current
          : [...current, id]
        : current.includes(id)
          ? current.filter((fieldId) => fieldId !== id)
          : current,
    );
  }, []);
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
    projection.relationshipDefinitionId ||
    projectionDrafts.length ||
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
    () => onController({ state: combinedState, submit, canSubmit: !formDirty, waitForPending }),
    [combinedState, submit, formDirty, onController, waitForPending],
  );
  const busy = disabled || fields.state === "saving";
  const hasValueDraft = Object.keys(fields.drafts).length > 0;
  const configDisabled = busy || hasValueDraft || !fields.snapshot;
  const selected = fields.snapshot?.definitions.find((d) => d.id === manageId);
  const presentedIds = [
    ...new Set(
      fields.snapshot?.fields
        .filter((field) => showHidden || !field.hidden)
        .flatMap(
          (field) => field.projectedRelationships?.map((relationship) => relationship.id) ?? [],
        ) ?? [],
    ),
  ]
    .sort()
    .join("\n");
  useEffect(() => {
    onPresentedRelationships?.(presentedIds ? presentedIds.split("\n") : []);
  }, [presentedIds, onPresentedRelationships]);
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
    setProjection({ relationshipDefinitionId: "", perspective: "source" });
    setFormError(null);
  }
  async function create() {
    try {
      if (newKind === "relationship") {
        await configure(
          { kind: "create_projection", name: newName, ...projection, provider: provider(newScope) },
          cancelNew,
        );
        return;
      }
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

  const manageDisabled =
    busy ||
    !!newName ||
    !!String(newValue) ||
    !!newOptions ||
    !!newUnit ||
    !!projection.relationshipDefinitionId ||
    !!projectionDrafts.length;
  useDesktopCommands(
    "entry-fields",
    {
      edit: [
        {
          id: "field",
          label: "Field",
          children: [
            {
              id: "field-add",
              label: "Add field…",
              disabled: configDisabled || !!renamed || !!optionLabel || !!projectionDrafts.length,
              action: () => setCreateOpen(true),
            },
            {
              id: "field-manage",
              label: "Manage fields…",
              disabled: manageDisabled,
              action: () => setManageOpen(true),
            },
            ...(fields.snapshot?.fields.some((field) => field.hidden)
              ? [
                  {
                    id: "field-hidden",
                    label: showHidden ? "Hide hidden fields" : "Show hidden fields",
                    checked: showHidden,
                    disabled: configDisabled || formDirty,
                    action: () => setShowHidden(!showHidden),
                  },
                ]
              : []),
          ],
        },
      ],
    },
    30,
  );

  return (
    <section aria-label="Entry fields" className="fields-panel">
      <div className="section-heading">
        <div>
          <h3>Fields</h3>
        </div>
        <div className="row panel-actions">
          <button
            disabled={configDisabled || !!renamed || !!optionLabel || !!projectionDrafts.length}
            onClick={() => setCreateOpen(true)}
          >
            Add field
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
      {hasValueDraft && fields.state === "failed" && (
        <button disabled={busy} onClick={() => void fields.submit()}>
          Retry saving fields
        </button>
      )}
      {fields.snapshot?.fields.length === 0 && (
        <p className="empty-state">No fields yet. A name and a value are enough to start.</p>
      )}
      {!manageOpen && (renamed || optionLabel) && (
        <p className="field-note">
          You have unfinished definition edits.{" "}
          <button
            className="quiet-button"
            onClick={() => {
              setManageOpen(true);
              setDefinitionOpen(true);
            }}
          >
            Continue definition edits
          </button>
        </p>
      )}
      {!createOpen &&
        (newName ||
          String(newValue) ||
          newOptions ||
          newUnit ||
          projection.relationshipDefinitionId) && (
          <p className="field-note">
            You have an unfinished Field.{" "}
            <button className="quiet-button" onClick={() => setCreateOpen(true)}>
              Continue Field draft
            </button>
          </p>
        )}
      <div className="field-grid">
        {fields.snapshot?.fields
          .filter((field) => showHidden || !field.hidden)
          .map((field) => (
            <div
              className={`field-value${field.hidden ? " hidden-field-preview" : ""}`}
              key={field.definition.id}
            >
              <div className="field-value-label">
                <label>{field.definition.name}</label>
                {field.hidden && <small>Hidden</small>}
                {!field.available && (
                  <small title="This value is kept even though its shared default is no longer active.">
                    Retained value
                  </small>
                )}
              </div>
              {field.definition.kind === "relationship" ? (
                <ProjectionFieldValue
                  projectId={projectId}
                  entryId={entry.id}
                  field={field}
                  disabled={
                    busy ||
                    hasValueDraft ||
                    !!newName ||
                    !!String(newValue) ||
                    !!newOptions ||
                    !!newUnit ||
                    !!projection.relationshipDefinitionId ||
                    !!renamed ||
                    !!optionLabel ||
                    projectionDrafts.some((id) => id !== field.definition.id)
                  }
                  onCommand={editProjection}
                  onDirty={updateProjectionDirty}
                  onNavigate={onNavigate}
                />
              ) : (
                <ValueInput
                  field={field}
                  draft={fields.drafts[field.definition.id] ?? valueDraft(field.value)}
                  disabled={disabled || formDirty || fields.configurationPending}
                  onChange={(draft) => fields.change(field.definition.id, draft)}
                  onBlur={() => void fields.submit()}
                />
              )}
              {field.definition.kind !== "relationship" &&
                (field.value !== null || fields.drafts[field.definition.id] !== undefined) && (
                  <button
                    className="quiet-button clear-field"
                    aria-label={`Clear value: ${field.definition.name}`}
                    disabled={disabled || formDirty || fields.configurationPending}
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
                setProjection({ relationshipDefinitionId: "", perspective: "source" });
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
          {newKind === "relationship" ? (
            <ProjectionConfiguration
              projectId={projectId}
              value={projection}
              onChange={setProjection}
            />
          ) : newKind === "choice" || newKind === "multi_choice" ? (
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
              disabled={
                !newName.trim() ||
                (newScope === "type" && !entry.typeId) ||
                (newKind === "relationship" && !projection.relationshipDefinitionId)
              }
              onClick={() => void create()}
            >
              Create field
            </button>
            <button onClick={cancelNew}>Cancel new field</button>
          </div>
        </fieldset>
      </Dialog>
      <Dialog
        open={manageOpen}
        title="Manage fields"
        onClose={() => setManageOpen(false)}
        className="field-manager-dialog"
      >
        <p className="muted">
          Choose a Field name to edit its shared definition. Hide keeps values; Delete affects only
          this Entry. Removing a relationship Field keeps its connections.
        </p>
        {manageOpen && !definitionOpen && !deleteReview && (fields.error || formError) && (
          <p role="alert">
            {fields.error || formError}{" "}
            <button disabled={busy} onClick={() => void fields.reload()}>
              Reload fields (keep drafts)
            </button>
          </p>
        )}
        {(renamed || optionLabel) && (
          <button onClick={() => setDefinitionOpen(true)}>Continue definition edits</button>
        )}
        {manageOpen && (
          <FieldManagerTables
            entry={entry}
            fields={fields.snapshot?.fields ?? []}
            definitions={fields.snapshot?.definitions ?? []}
            disabled={configDisabled || formDirty}
            onEdit={(d) => {
              setManageId(d.id);
              setOptionId("");
              setDefinitionOpen(true);
            }}
            onHide={(field) =>
              void configure({
                kind: "set_hidden",
                fieldId: field.definition.id,
                hidden: !field.hidden,
              })
            }
            onDelete={(field) => {
              setFormError(null);
              if (field.definition.kind === "relationship") {
                void configure({ kind: "remove_projection", fieldId: field.definition.id });
                return;
              }
              setDeleteReview({
                field,
                revision: Math.max(fields.snapshot!.globalRevision, getRevision?.() ?? 0),
              });
            }}
            onAdd={(d) =>
              void configure({ kind: "bind", fieldId: d.id, provider: provider("entry") })
            }
          />
        )}
      </Dialog>
      {deleteReview && (
        <DeleteEntryFieldDialog
          field={deleteReview.field}
          entryName={entry.displayName}
          busy={busy}
          error={fields.error}
          onClose={() => setDeleteReview(null)}
          onDelete={() =>
            void fields
              .deleteLocal(
                deleteReview.field.definition.id,
                deleteReview.revision,
                onRecoveryBackup,
              )
              .then((result) => {
                if (result.kind === "committed") setDeleteReview(null);
              })
          }
        />
      )}
      <Dialog
        open={definitionOpen && manageOpen}
        title="Edit field definition"
        onClose={() => setDefinitionOpen(false)}
      >
        <p className="muted">
          Changes here affect every Entry using this definition. To combine duplicates, open
          Categories → Field maintenance.
        </p>
        {manageOpen && definitionOpen && (fields.error || formError) && (
          <p role="alert">
            {fields.error || formError}{" "}
            <button disabled={busy} onClick={() => void fields.reload()}>
              Reload fields (keep drafts)
            </button>
          </p>
        )}
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
            <details className="manager-secondary">
              <summary>Availability and retirement</summary>
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
                    {b.label}{" "}
                    <button
                      disabled={!!renamed || !!optionLabel}
                      onClick={() =>
                        void configure({
                          kind: "unbind",
                          fieldId: selected.id,
                          provider: b.provider,
                        })
                      }
                    >
                      Detach from {b.label}
                    </button>
                  </li>
                ))}
              </ul>
            </details>
            {(selected.kind === "choice" || selected.kind === "multi_choice") && (
              <>
                <ManagerSearchSelect
                  label="Choice option"
                  ariaLabel="choice-option"
                  disabled={!!optionLabel}
                  value={optionId}
                  onChange={setOptionId}
                  emptyLabel="Add a new option"
                  choices={selected.options.map((option) => ({
                    id: option.id,
                    label: `${option.label}${option.retired ? " (retired)" : ""}`,
                  }))}
                />
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
