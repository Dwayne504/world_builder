import { useCallback, useEffect, useRef, useState } from "react";
import {
  applyTemplateFields,
  createCategory,
  createType,
  listCategories,
  listTypes,
  readFieldCatalog,
} from "./api";
import { Dialog } from "./Dialog";
import type { FieldsController } from "./EntryFieldsPanel";
import type {
  Category,
  FieldCatalog,
  FieldCommand,
  FieldKind,
  FieldProvider,
  SaveState,
  TypeDef,
} from "./types";
import type { SubmitOutcome } from "./useProjectRename";

const kinds: Record<FieldKind, string> = {
  short_text: "Short Text",
  number: "Number (optional unit)",
  boolean: "Boolean",
  choice: "Choice",
  multi_choice: "Multi-choice",
};

export function CategoryManager({
  projectId,
  open,
  onClose,
  onController,
  onChanged,
}: {
  projectId: string;
  open: boolean;
  onClose: () => void;
  onController: (controller: FieldsController) => void;
  onChanged: (revision: number) => void;
}) {
  const [categories, setCategories] = useState<Category[]>([]);
  const [types, setTypes] = useState<TypeDef[]>([]);
  const [catalog, setCatalog] = useState<FieldCatalog | null>(null);
  const [categoryId, setCategoryId] = useState("");
  const [typeId, setTypeId] = useState("");
  const [categoryName, setCategoryName] = useState("");
  const [typeName, setTypeName] = useState("");
  const [parentId, setParentId] = useState("");
  const [fieldName, setFieldName] = useState("");
  const [fieldKind, setFieldKind] = useState<FieldKind>("short_text");
  const [unit, setUnit] = useState("");
  const [options, setOptions] = useState("");
  const [existingField, setExistingField] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState<"category" | "type" | "field" | "reuse" | null>(null);
  const pending = useRef<Promise<SubmitOutcome> | null>(null);
  const generation = useRef(0);
  const changedRef = useRef(onChanged);
  changedRef.current = onChanged;
  const fieldDirty = !!(fieldName || unit || options);
  const dirty = !!(categoryName || typeName || fieldDirty);
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  const busy = saveState === "saving";
  const selectedCategory = categories.find((c) => c.id === categoryId);
  const categoryTypes = types.filter((t) => t.categoryId === categoryId);
  const provider: FieldProvider = { kind: typeId ? "type" : "category", id: typeId || categoryId };
  const targetLabel =
    categoryTypes.find((t) => t.id === typeId)?.name ?? selectedCategory?.name ?? "";
  const definitions = catalog?.definitions ?? [];
  const supplied = definitions.filter((d) =>
    d.bindings.some((b) => b.provider.kind === provider.kind && b.provider.id === provider.id),
  );
  const reload = useCallback(async () => {
    const request = ++generation.current;
    setLoading(true);
    try {
      const [nextCategories, nextCatalog] = await Promise.all([
        listCategories(projectId),
        readFieldCatalog(projectId),
      ]);
      const nextTypes = (
        await Promise.all(nextCategories.map((c) => listTypes(projectId, c.id)))
      ).flat();
      if (request !== generation.current) return;
      setCategories(nextCategories);
      setTypes(nextTypes);
      setCatalog(nextCatalog);
      setCategoryId((id) =>
        nextCategories.some((c) => c.id === id) ? id : (nextCategories[0]?.id ?? ""),
      );
      setError(null);
      setSaveState("saved");
    } catch (reason) {
      if (request === generation.current)
        setError(reason instanceof Error ? reason.message : "Categories could not be loaded.");
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [projectId]);
  useEffect(() => {
    if (open && !pending.current) void reload();
  }, [open, reload]);
  const invalidate = useCallback(() => {
    ++generation.current;
  }, []);
  useEffect(() => invalidate, [invalidate]);

  const submit = useCallback(async (): Promise<SubmitOutcome> => {
    const result = await (pending.current ?? Promise.resolve({ kind: "no-op" } as SubmitOutcome));
    if (result.kind === "failed") return result;
    if (dirtyRef.current) {
      setError("Finish or cancel the Category manager draft before leaving.");
      return { kind: "failed" };
    }
    return result;
  }, []);
  const state = busy || saveState === "failed" ? saveState : dirty ? "dirty" : "saved";
  useEffect(
    () => onController({ state, submit, canSubmit: !dirty }),
    [state, submit, dirty, onController],
  );
  function resetDrafts() {
    setForm(null);
    setSaveState("saved");
    setCategoryName("");
    setTypeName("");
    setParentId("");
    setFieldName("");
    setUnit("");
    setOptions("");
    dirtyRef.current = false;
    setError(null);
  }
  function perform(action: () => Promise<{ globalRevision: number }>, after?: () => void) {
    if (pending.current || loading) return;
    ++generation.current;
    setSaveState("saving");
    setError(null);
    pending.current = action()
      .then((result): SubmitOutcome => {
        resetDrafts();
        after?.();
        setSaveState("saved");
        changedRef.current(result.globalRevision);
        // A failed refresh cannot turn an acknowledged creation into a failed
        // creation or invite a duplicate retry. Its draft was already committed.
        void reload();
        return { kind: "committed" };
      })
      .catch((reason: unknown): SubmitOutcome => {
        setSaveState("failed");
        setError(reason instanceof Error ? reason.message : "The change could not be saved.");
        return { kind: "failed" };
      })
      .finally(() => {
        pending.current = null;
      });
  }
  function apply(command: FieldCommand) {
    if (catalog)
      perform(
        () => applyTemplateFields(projectId, catalog.globalRevision, command),
        () => setExistingField(""),
      );
  }
  const activeDraft = fieldDirty ? "field" : typeName ? "type" : categoryName ? "category" : null;
  const dismissForm = () => setForm(null);
  const formFeedback = (
    <>
      {error && (
        <p role="alert" className="error-banner">
          {error}{" "}
          <button disabled={busy} onClick={() => void reload()}>
            Reload Categories (keep draft)
          </button>
        </p>
      )}
      {loading && <p role="status">Loading Categories…</p>}
    </>
  );
  const cancelButton = (
    <button disabled={busy} onClick={resetDrafts}>
      Cancel manager draft
    </button>
  );
  return (
    <>
      {!open && dirty && (
        <p className="field-note">
          Category manager has an unfinished draft. Open Categories to continue or cancel it.
        </p>
      )}
      {!open && error && <p role="alert">{error}</p>}
      <Dialog
        open={open}
        title="Categories and defaults"
        onClose={onClose}
        className="category-dialog"
      >
        <p>
          Organize your world. Choose a Category to manage its Types and the optional Fields that
          appear on its Entries.
        </p>
        {!form && formFeedback}
        {!form && dirty && (
          <p className="field-note">
            You have an unfinished draft.{" "}
            <button onClick={() => setForm(activeDraft)}>Continue manager draft</button>{" "}
            {cancelButton}
          </p>
        )}
        <div className="category-workspace">
          <div className="category-sidebar">
            <label>
              Category
              <select
                aria-label="Managed Category"
                disabled={busy || dirty}
                value={categoryId}
                onChange={(e) => {
                  setCategoryId(e.target.value);
                  setTypeId("");
                  setParentId("");
                  setExistingField("");
                }}
              >
                <option value="">Choose a Category</option>
                {categories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </label>
            <button
              disabled={busy || loading || (dirty && activeDraft !== "category")}
              onClick={() => setForm("category")}
            >
              Add Category
            </button>
          </div>
          <div className="category-main">
            {selectedCategory ? (
              <>
                <div className="section-heading">
                  <h3>Types in {selectedCategory.name}</h3>
                  <button
                    disabled={busy || loading || (dirty && activeDraft !== "type")}
                    onClick={() => setForm("type")}
                  >
                    Add Type
                  </button>
                </div>
                {categoryTypes.length ? (
                  <ul className="manager-types">
                    {categoryTypes.map((t) => (
                      <li key={t.id}>
                        <span>
                          {t.name}
                          {t.parentTypeId && (
                            <small>
                              {" "}
                              · {categoryTypes.find((p) => p.id === t.parentTypeId)?.name}
                            </small>
                          )}
                        </span>
                        <button
                          className="quiet-button"
                          disabled={busy || dirty}
                          onClick={() => {
                            setTypeId(t.id);
                            setExistingField("");
                          }}
                        >
                          Defaults for {t.name}
                        </button>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="muted">
                    No Types yet. Entries can also belong directly to this Category.
                  </p>
                )}
                <div className="section-heading default-heading">
                  <h3>Default fields</h3>
                  <div className="row">
                    <button
                      disabled={busy || loading || (dirty && activeDraft !== "field") || !catalog}
                      onClick={() => setForm("field")}
                    >
                      Add default field
                    </button>
                    <button
                      className="quiet-button"
                      disabled={busy || loading || dirty || !catalog}
                      onClick={() => setForm("reuse")}
                    >
                      Reuse field
                    </button>
                  </div>
                </div>
                <label>
                  Defaults for
                  <select
                    aria-label="Default field scope"
                    disabled={busy || dirty}
                    value={typeId}
                    onChange={(e) => {
                      setTypeId(e.target.value);
                      setExistingField("");
                    }}
                  >
                    <option value="">All {selectedCategory.name} Entries</option>
                    {categoryTypes.map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.name} Entries
                      </option>
                    ))}
                  </select>
                </label>
                {typeId && (
                  <p className="muted">
                    Category and parent-Type defaults are also available. The list below configures
                    this Type's own defaults.
                  </p>
                )}
                <ul className="template-fields">
                  {supplied.map((d) => (
                    <li key={d.id}>
                      <span>
                        {d.name}{" "}
                        <small>
                          {d.kind === "number" ? "Number" : kinds[d.kind]}
                          {d.unit ? ` · ${d.unit}` : ""}
                          {d.retired ? " · retired" : ""}
                        </small>
                      </span>
                      <button
                        className="quiet-button"
                        disabled={busy || dirty || loading}
                        onClick={() => apply({ kind: "unbind", fieldId: d.id, provider })}
                      >
                        Remove default: {d.name}
                      </button>
                    </li>
                  ))}
                </ul>
                {!supplied.length && <p className="empty-state">No fields configured here yet.</p>}
                <p className="field-note">
                  Adding a default makes it available on existing and future matching Entries.
                  Removing a default preserves values already filled in.
                </p>
              </>
            ) : (
              <p className="empty-state">Choose a Category or add one to get started.</p>
            )}
          </div>
        </div>
      </Dialog>
      <Dialog open={open && form === "category"} title="Add Category" onClose={dismissForm}>
        <p>A broad home for Entries, such as Characters, Places, or Weapons.</p>
        {form === "category" && formFeedback}
        <fieldset disabled={busy || loading || !!typeName || fieldDirty}>
          <label>
            Category name
            <input
              aria-label="Category manager name"
              value={categoryName}
              onChange={(e) => setCategoryName(e.target.value)}
            />
          </label>
          <button
            disabled={!categoryName.trim()}
            onClick={() =>
              perform(async () => {
                const next = await createCategory(projectId, categoryName);
                setCategoryId(next.id);
                setTypeId("");
                return next;
              })
            }
          >
            Create Category
          </button>
        </fieldset>
        {cancelButton}
      </Dialog>
      <Dialog open={open && form === "type"} title="Add Type" onClose={dismissForm}>
        <p>Create a Type in {selectedCategory?.name}.</p>
        {form === "type" && formFeedback}
        <fieldset disabled={busy || loading || !!categoryName || fieldDirty}>
          <label>
            Type name
            <input
              aria-label="Category manager Type name"
              value={typeName}
              onChange={(e) => setTypeName(e.target.value)}
            />
          </label>
          <label>
            Parent Type (optional)
            <select
              aria-label="Parent Type"
              value={parentId}
              onChange={(e) => setParentId(e.target.value)}
            >
              <option value="">No parent</option>
              {categoryTypes.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          <button
            disabled={!typeName.trim()}
            onClick={() =>
              perform(() => createType(projectId, categoryId, typeName, parentId || undefined))
            }
          >
            Create Type
          </button>
        </fieldset>
        {cancelButton}
      </Dialog>
      <Dialog open={open && form === "field"} title="Add default field" onClose={dismissForm}>
        <p>
          Make an optional Field available to {targetLabel} Entries. Each Entry keeps its own value.
        </p>
        {form === "field" && formFeedback}
        <fieldset disabled={busy || loading || !catalog || !!categoryName || !!typeName}>
          <label>
            Field name
            <input
              aria-label="Default field name"
              value={fieldName}
              onChange={(e) => setFieldName(e.target.value)}
            />
          </label>
          <label>
            Kind
            <select
              aria-label="Default field kind"
              value={fieldKind}
              onChange={(e) => {
                setFieldKind(e.target.value as FieldKind);
                setOptions("");
                setUnit("");
              }}
            >
              {Object.entries(kinds).map(([kind, label]) => (
                <option key={kind} value={kind}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          {fieldKind === "number" && (
            <label>
              Unit (optional)
              <input
                aria-label="Default field unit"
                placeholder="tons, km, years, gold crowns..."
                value={unit}
                onChange={(e) => setUnit(e.target.value)}
              />
            </label>
          )}
          {(fieldKind === "choice" || fieldKind === "multi_choice") && (
            <label>
              Options (one per line)
              <textarea
                aria-label="Default field options"
                value={options}
                onChange={(e) => setOptions(e.target.value)}
              />
            </label>
          )}
          <button
            disabled={!fieldName.trim()}
            onClick={() =>
              apply({
                kind: "create",
                name: fieldName,
                fieldKind,
                ...(fieldKind === "number" && unit.trim() ? { unit: unit.trim() } : {}),
                provider,
                options: options
                  .split("\n")
                  .map((s) => s.trim())
                  .filter(Boolean),
                value: null,
              })
            }
          >
            Create default field
          </button>
        </fieldset>
        {cancelButton}
      </Dialog>
      <Dialog open={open && form === "reuse"} title="Reuse field" onClose={dismissForm}>
        <p>Use the same Field definition for {targetLabel} Entries.</p>
        {form === "reuse" && formFeedback}
        <label>
          Existing field
          <select
            aria-label="Existing default field"
            disabled={busy || dirty || loading}
            value={existingField}
            onChange={(e) => setExistingField(e.target.value)}
          >
            <option value="">Choose a field</option>
            {definitions
              .filter((d) => !d.retired && !supplied.some((s) => s.id === d.id))
              .map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                  {d.unit ? ` (${d.unit})` : ""} · {kinds[d.kind]}
                  {definitions.filter((other) => other.name === d.name).length > 1
                    ? ` · ${d.id.slice(-8)}`
                    : ""}
                </option>
              ))}
          </select>
        </label>
        <button
          disabled={busy || dirty || loading || !existingField}
          onClick={() => apply({ kind: "bind", fieldId: existingField, provider })}
        >
          Use as default
        </button>

        <button
          disabled={busy}
          onClick={() => {
            setExistingField("");
            setForm(null);
          }}
        >
          Cancel
        </button>
      </Dialog>
    </>
  );
}
