import type { Entry, EntryField, FieldDefinition } from "./types";
import { fieldKinds, fieldLabel } from "./fieldLabels";

export function FieldManagerTables({
  entry,
  fields,
  definitions,
  disabled,
  onEdit,
  onHide,
  onDelete,
  onAdd,
}: {
  entry: Entry;
  fields: EntryField[];
  definitions: FieldDefinition[];
  disabled: boolean;
  onEdit: (field: FieldDefinition) => void;
  onHide: (field: EntryField) => void;
  onDelete: (field: EntryField) => void;
  onAdd: (field: FieldDefinition) => void;
}) {
  const others = definitions.filter((d) => !fields.some((f) => f.definition.id === d.id));
  function defaultLabel(field: EntryField) {
    if (!field.available) return "—";
    const sources =
      field.defaultSources ??
      field.definition.bindings.filter(
        (b) =>
          (b.provider.kind === "category" && b.provider.id === entry.categoryId) ||
          (b.provider.kind === "type" && b.provider.id === entry.typeId),
      );
    return (
      sources
        .map((b) => `${b.provider.kind === "type" ? "Type" : "Category"}: ${b.label}`)
        .join(", ") || "—"
    );
  }
  return (
    <div className="field-manager-columns">
      <div>
        <h3>
          On this Entry <small>({fields.length})</small>
        </h3>
        <div className="manager-table-scroll">
          <table className="field-manager-table">
            <thead>
              <tr>
                <th>Field</th>
                <th>Default</th>
                <th>
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {fields.map((field) => (
                <tr
                  key={field.definition.id}
                  className={field.hidden ? "hidden-field-row" : undefined}
                >
                  <th scope="row">
                    <button
                      className="field-name-button"
                      disabled={disabled}
                      title={fieldLabel(field.definition, definitions)}
                      aria-label={`Edit definition: ${field.definition.name}`}
                      onClick={() => onEdit(field.definition)}
                    >
                      {field.definition.name}
                    </button>
                    <small>
                      {field.definition.unit || fieldKinds[field.definition.kind]}
                      {field.hidden ? " · hidden" : ""}
                      {field.definition.retired ? " · retired" : ""}
                    </small>
                  </th>
                  <td>{defaultLabel(field)}</td>
                  <td>
                    <div className="field-row-actions">
                      <button
                        className="quiet-button"
                        disabled={disabled}
                        aria-label={`${field.hidden ? "Show" : "Hide"} field: ${field.definition.name}`}
                        onClick={() => onHide(field)}
                      >
                        {field.hidden ? "Show" : "Hide"}
                      </button>
                      <button
                        className="quiet-button"
                        disabled={disabled}
                        aria-label={`Delete from Entry: ${field.definition.name}`}
                        onClick={() => onDelete(field)}
                      >
                        Delete
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!fields.length && <p className="muted">No Fields on this Entry yet.</p>}
      </div>
      <div>
        <h3>Other fields &amp; defaults</h3>
        <div className="manager-table-scroll">
          <table className="field-manager-table">
            <thead>
              <tr>
                <th>Field</th>
                <th>Available from</th>
                <th>
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {others.map((d) => (
                <tr key={d.id}>
                  <th scope="row">
                    <button
                      className="field-name-button"
                      disabled={disabled}
                      title={fieldLabel(d, definitions)}
                      aria-label={`Edit definition: ${d.name}`}
                      onClick={() => onEdit(d)}
                    >
                      {d.name}
                    </button>
                    <small>{d.unit || fieldKinds[d.kind]}</small>
                  </th>
                  <td>
                    {d.retired
                      ? "Retired"
                      : d.bindings
                          .filter((b) => b.provider.kind !== "entry")
                          .map((b) => b.label)
                          .join(", ") || "Custom field"}
                  </td>
                  <td>
                    <button
                      className="quiet-button"
                      disabled={disabled || d.retired}
                      aria-label={`Add to Entry: ${d.name}`}
                      onClick={() => onAdd(d)}
                    >
                      Add
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!others.length && <p className="muted">All available Fields are already listed here.</p>}
      </div>
    </div>
  );
}
