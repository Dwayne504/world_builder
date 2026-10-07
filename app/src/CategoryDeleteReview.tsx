import { useEffect, useState } from "react";
import { previewCategoryDelete } from "./api";
import type { Category, CategoryDeletePreview, StructureCommand } from "./types";
import { ManagerSearchSelect } from "./ManagerSearchSelect";

export function CategoryDeleteReview({
  projectId,
  category,
  categories,
  busy,
  onConfirm,
  onCancel,
}: {
  projectId: string;
  category: Category;
  categories: Category[];
  busy: boolean;
  onConfirm: (revision: number, command: StructureCommand) => void;
  onCancel: () => void;
}) {
  const [preview, setPreview] = useState<CategoryDeletePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [destination, setDestination] = useState("");
  const [removeTypes, setRemoveTypes] = useState(false);
  useEffect(() => {
    let current = true;
    setPreview(null);
    setError(null);
    setRemoveTypes(false);
    void previewCategoryDelete(projectId, category.id)
      .then((value) => {
        if (current) setPreview(value);
      })
      .catch((e: unknown) => {
        if (current) setError(e instanceof Error ? e.message : "Could not review this Category.");
      });
    return () => {
      current = false;
    };
  }, [projectId, category.id, attempt]);
  return (
    <>
      {error && <p role="alert">{error}</p>}
      {!preview && !error && <p role="status">Reviewing Category…</p>}
      {preview && (
        <>
          <p>
            Delete <strong>{preview.name}</strong>? Its {preview.entryCount} Entries, including any
            in Archive or Trash, will be moved to the Category you choose. Their values, links and
            features stay intact.
          </p>
          <ManagerSearchSelect
            label="Move Entries to"
            value={destination}
            onChange={setDestination}
            disabled={busy}
            emptyLabel="Choose a destination"
            choices={categories
              .filter((c) => c.id !== category.id)
              .map((c) => ({ id: c.id, label: c.name }))}
          />
          {!!preview.typeNames.length && (
            <>
              <p>These Types belong to this Category: {preview.typeNames.join(", ")}.</p>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={removeTypes}
                  disabled={busy}
                  onChange={(e) => setRemoveTypes(e.target.checked)}
                />
                Remove these Types and clear the Type on {preview.typedEntryCount} Entries.
                Filled-in values are kept.
              </label>
            </>
          )}
          <p className="muted">
            This also removes the Category's default settings. Shared Field definitions and existing
            values remain. A recovery copy is saved automatically before deletion.
          </p>
          <button
            disabled={busy || !destination || (!!preview.typeNames.length && !removeTypes)}
            onClick={() =>
              onConfirm(preview.globalRevision, {
                kind: "delete_category",
                id: category.id,
                destinationId: destination,
                removeTypes,
              })
            }
          >
            Delete Category
          </button>
        </>
      )}
      <button className="quiet-button" disabled={busy} onClick={() => setAttempt((n) => n + 1)}>
        Refresh review
      </button>
      <button disabled={busy} onClick={onCancel}>
        Cancel
      </button>
    </>
  );
}
