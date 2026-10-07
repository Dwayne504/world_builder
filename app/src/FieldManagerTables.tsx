import type { Entry, EntryField, FieldDefinition } from "./types";
import { fieldKinds, fieldLabel } from "./fieldLabels";
import { useState } from "react";
import "./ManagerWorkspace.css";

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
  const [search, setSearch] = useState("");
  const [currentLimit, setCurrentLimit] = useState(12);
  const [otherLimit, setOtherLimit] = useState(12);
  const others = definitions.filter((d) => !fields.some((f) => f.definition.id === d.id));
  const query = search.trim().toLocaleLowerCase();
  const matches = (definition: FieldDefinition) =>
    `${definition.name} ${fieldKinds[definition.kind]} ${definition.unit ?? ""} ${definition.bindings.map((binding) => binding.label).join(" ")}`
      .toLocaleLowerCase()
      .includes(query);
  const matchingFields = fields.filter((field) => matches(field.definition));
  const matchingOthers = others.filter(matches);
  function defaultLabel(field: EntryField) {
    if (!field.available) return "—";
    const sources =
      field.defaultSources ??
      field.definition.bindings.filter(
        (b) =>
          (b.provider.kind === "category" && b.provider.id === entry.categoryId) ||
          (b.provider.kind === "type" && b.provider.id === entry.typeId),
      );
    return sources.map((b) => b.label).join(", ") || "—";
  }
  return (
    <div className="field-manager-workspace">
      <div className="manager-list-toolbar">
        <label>
          Find a field
          <input
            type="search"
            placeholder="Name, kind, unit, or default"
            value={search}
            onChange={(event) => {
              setSearch(event.currentTarget.value);
              setCurrentLimit(12);
              setOtherLimit(12);
            }}
          />
        </label>
        <span className="manager-count">
          {matchingFields.length + matchingOthers.length} fields
          {query ? " matching" : " available to browse"}
        </span>
      </div>
      <div className="field-manager-columns">
        <div>
          <h3>
            On this Entry{" "}
            <small>
              ({matchingFields.length}
              {query ? ` of ${fields.length}` : ""})
            </small>
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
                {matchingFields.slice(0, currentLimit).map((field) => (
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
                          disabled={
                            disabled ||
                            !!(
                              field.value?.kind === "rich_text" && field.value.value.readOnlyReason
                            )
                          }
                          aria-label={`${field.definition.kind === "relationship" ? "Remove from Fields" : "Delete from Entry"}: ${field.definition.name}`}
                          title={
                            field.definition.kind === "relationship"
                              ? "Remove this display only; keep the relationships."
                              : undefined
                          }
                          onClick={() => onDelete(field)}
                        >
                          {field.definition.kind === "relationship"
                            ? "Remove from Fields"
                            : "Delete"}
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!fields.length && <p className="muted">No Fields on this Entry yet.</p>}
          {!!fields.length && !matchingFields.length && (
            <p className="muted">No fields on this Entry match your search.</p>
          )}
          {matchingFields.length > currentLimit && (
            <div className="manager-list-footer">
              <small>
                Showing {currentLimit} of {matchingFields.length}
              </small>
              <button className="quiet-button" onClick={() => setCurrentLimit(currentLimit + 12)}>
                Show more Entry fields
              </button>
            </div>
          )}
        </div>
        <div>
          <h3>
            Other fields &amp; defaults{" "}
            <small>
              ({matchingOthers.length}
              {query ? ` of ${others.length}` : ""})
            </small>
          </h3>
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
                {matchingOthers.slice(0, otherLimit).map((d) => (
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
          {!!others.length && !matchingOthers.length && (
            <p className="muted">No other fields match your search.</p>
          )}
          {matchingOthers.length > otherLimit && (
            <div className="manager-list-footer">
              <small>
                Showing {otherLimit} of {matchingOthers.length}
              </small>
              <button className="quiet-button" onClick={() => setOtherLimit(otherLimit + 12)}>
                Show more reusable fields
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
