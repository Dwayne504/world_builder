import { Dialog } from "./Dialog";
import type { EntryField } from "./types";

export function DeleteEntryFieldDialog({
  field,
  entryName,
  busy,
  error,
  onClose,
  onDelete,
}: {
  field: EntryField;
  entryName: string;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onDelete: () => void;
}) {
  const value = field.value;
  const description =
    value?.kind === "rich_text"
      ? value.value.plainText || "Preserved Rich Text"
      : value?.kind === "choices"
        ? value.value
            .map(
              (id) => field.definition.options.find((o) => o.id === id)?.label ?? "Unknown option",
            )
            .join(", ")
        : value?.kind === "boolean"
          ? value.value
            ? "Yes"
            : "No"
          : value
            ? String(value.value)
            : "Not filled in";
  return (
    <Dialog open title={`Delete ${field.definition.name} from this Entry?`} onClose={onClose}>
      <p>
        This removes <strong>{field.definition.name}</strong> and its value from{" "}
        <strong>{entryName}</strong> only. Other Entries and shared defaults stay unchanged.
      </p>
      <p className="delete-value-preview">
        Current value: {description}
        {value && field.definition.unit ? ` ${field.definition.unit}` : ""}
      </p>
      <p>
        You can add the Field back empty later. To keep its current value out of sight, use Hide
        instead.
      </p>
      <div className="row">
        <button
          disabled={busy || !!(value?.kind === "rich_text" && value.value.readOnlyReason)}
          onClick={onDelete}
        >
          Delete
        </button>
        <button disabled={busy} onClick={onClose}>
          Cancel
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
      {busy && <p role="status">Deleting…</p>}
    </Dialog>
  );
}
