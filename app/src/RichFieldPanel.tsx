import { useCallback, useEffect, useRef } from "react";
import { applyFields, readFields } from "./api";
import { EntryDocumentPanel } from "./EntryDocumentPanel";
import { useEntryDocument } from "./useEntryDocument";
import type { FieldsController } from "./EntryFieldsPanel";
import type { Entry, EntryField, EntryFields } from "./types";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";

export function RichFieldPanel({
  projectId,
  entry,
  field,
  disabled,
  onController,
  onRevision,
  getRevision,
  onCommitted,
}: {
  projectId: string;
  entry: Entry;
  field: EntryField;
  disabled: boolean;
  onController: (id: string, controller: FieldsController | null) => void;
  onRevision: (revision: number) => void;
  getRevision: () => number;
  onCommitted: (revision: number) => void;
}) {
  const fieldId = field.definition.id;
  // Each writing area has an isolated emergency recovery key.
  const recoveryId = `${entry.id}:field:${fieldId}`;
  const snapshot = (fields: EntryFields): EntryDescriptionSnapshot => {
    const current = fields.fields.find((value) => value.definition.id === fieldId);
    if (
      !fields.definitions.some(
        (definition) => definition.id === fieldId && definition.kind === "rich_text",
      )
    )
      throw new Error("This Rich Text Field is no longer available. Your draft is preserved.");
    const value = current?.value?.kind === "rich_text" ? current.value.value : null;
    return {
      globalRevision: fields.globalRevision,
      entryId: recoveryId,
      workspaceState: entry.workspaceState,
      document: value
        ? {
            ...value,
            id: fieldId,
            wordCount: value.plainText.trim().split(/\s+/).filter(Boolean).length,
          }
        : null,
    };
  };
  const writing = useEntryDocument(projectId, recoveryId, onRevision, getRevision, {
    read: async () => snapshot(await readFields(projectId, entry.id)),
    save: async (expected, revision, version, content) => {
      const next = await applyFields(projectId, entry.id, expected, {
        kind: "set_values",
        edits: [
          {
            fieldId,
            value: {
              kind: "rich_text",
              value: {
                schemaVersion: version,
                content,
                revision: revision ?? 0,
                plainText: "",
                readOnlyReason: null,
                originalJson: null,
              },
            },
          },
        ],
      });
      onCommitted(next.globalRevision);
      return snapshot(next);
    },
  });
  const register = useCallback(
    (controller: FieldsController) => onController(fieldId, controller),
    [fieldId, onController],
  );
  const cleanup = useRef(onController);
  cleanup.current = onController;
  useEffect(() => () => cleanup.current(fieldId, null), [fieldId]);
  return (
    <EntryDocumentPanel
      description={writing}
      label={`Value: ${field.definition.name}`}
      noun="writing"
      alwaysOpen
      disabled={disabled}
      onController={register}
    />
  );
}
