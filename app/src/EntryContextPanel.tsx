import { useEffect, useRef, useState } from "react";
import {
  getEntry,
  listTypes,
  readEntryDescription,
  readFields,
  readRelationships,
  readSpatial,
} from "./api";
import type {
  Category,
  Entry,
  EntryField,
  EntryFields,
  EntryRelationships,
  SpatialSnapshot,
} from "./types";
import type { EntryDescriptionSnapshot } from "./entryDescriptionTypes";
import { spatialPath } from "./spatialPresentation";
import "./EntryContextPanel.css";

function FieldPreview({
  field,
  onPreview,
}: {
  field: EntryField;
  onPreview?: (id: string) => void;
}) {
  const value = field.value;
  if (value?.kind === "entry_references")
    return (
      <>
        {field.referenceTargets?.map((target, index) => (
          <span key={target.id}>
            {index > 0 && ", "}
            {target.targetId && onPreview ? (
              <button className="text-link" onClick={() => onPreview(target.targetId!)}>
                {target.label}
              </button>
            ) : (
              target.label
            )}
            {target.workspaceState && target.workspaceState !== "active"
              ? ` (${target.workspaceState})`
              : ""}
          </span>
        ))}
      </>
    );
  if (value?.kind === "rich_text")
    return (
      <>
        {value.value.plainText}
        {value.value.readOnlyReason && <small className="muted"> · Needs recovery</small>}
      </>
    );
  if (value?.kind === "choices")
    return (
      <>
        {value.value
          .map(
            (id) =>
              field.definition.options.find((option) => option.id === id)?.label ??
              "Unavailable choice",
          )
          .join(", ")}
      </>
    );
  if (value?.kind === "boolean") return <>{value.value ? "Yes" : "No"}</>;
  return (
    <>
      {value?.value}
      {value?.kind === "number" && field.definition.unit ? ` ${field.definition.unit}` : ""}
    </>
  );
}

/** Read-only, committed context. Never registers a save controller or mounts another editor. */
export function EntryContextContent({
  projectId,
  entryId,
  categories = [],
  onPreview,
  onLoaded,
}: {
  projectId: string;
  entryId: string;
  categories?: Category[];
  onPreview?: (id: string) => void;
  onLoaded?: (entry: Entry) => void;
}) {
  const [content, setContent] = useState<{
    entry: Entry;
    type: string | undefined;
    description: EntryDescriptionSnapshot;
    fields: EntryFields;
    relationships: EntryRelationships;
    spatial: SpatialSnapshot;
  } | null>(null);
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const loaded = useRef(onLoaded);
  loaded.current = onLoaded;
  useEffect(() => {
    let current = true;
    setContent(null);
    setError(false);
    void getEntry(projectId, entryId)
      .then(async (entry) => {
        const [description, fields, relationships, spatial, types] = await Promise.all([
          readEntryDescription(projectId, entryId),
          readFields(projectId, entryId),
          readRelationships(projectId, entryId),
          readSpatial(projectId),
          entry.typeId ? listTypes(projectId, entry.categoryId) : Promise.resolve([]),
        ]);
        if (current) {
          setContent({
            entry,
            description,
            fields,
            relationships,
            spatial,
            type: types.find((type) => type.id === entry.typeId)?.name,
          });
          loaded.current?.(entry);
        }
      })
      .catch(() => {
        if (current) setError(true);
      });
    return () => {
      current = false;
    };
  }, [projectId, entryId, attempt]);
  if (error)
    return (
      <div role="alert">
        <p>This reference could not be loaded. Your writing is still here.</p>
        <button onClick={() => setAttempt((value) => value + 1)}>Retry reference</button>
      </div>
    );
  if (!content) return <p role="status">Loading reference…</p>;
  const { entry, description, fields, relationships } = content;
  const facts = fields.fields.filter((field) => field.value && !field.hidden);
  const connections = relationships.relationships.filter(
    (relationship) => !relationship.ended && relationship.workspaceState === "active",
  );
  const path = spatialPath(content.spatial.entries, entryId);
  return (
    <div className="entry-context-content">
      <p className="muted">
        {[
          categories.find((category) => category.id === entry.categoryId)?.name,
          content.type,
          entry.workspaceState !== "active" ? entry.workspaceState : null,
        ]
          .filter(Boolean)
          .join(" · ")}
      </p>
      {description.document?.readOnlyReason && (
        <p role="status">{description.document.readOnlyReason}</p>
      )}
      <div className="entry-context-prose">
        {description.document?.plainText || <p className="muted">No description yet.</p>}
      </div>
      {path.length > 1 && <p className="muted">{path.map((place) => place.label).join(" › ")}</p>}
      {facts.length > 0 && (
        <details className="entry-context-facts">
          <summary>
            Facts <small>{facts.length}</small>
          </summary>
          <dl>
            {facts.slice(0, 12).map((field) => (
              <div key={field.definition.id}>
                <dt>{field.definition.name}</dt>
                <dd>
                  <FieldPreview field={field} onPreview={onPreview} />
                </dd>
              </div>
            ))}
          </dl>
          {facts.length > 12 && (
            <p className="muted">Open the Entry to see all {facts.length} facts.</p>
          )}
        </details>
      )}
      {connections.length > 0 && (
        <section aria-label="Connected Entries">
          <h4>Connected Entries</h4>
          {connections.slice(0, 8).map((relationship) => {
            const source = relationship.source.id === entryId;
            const other = source ? relationship.target : relationship.source;
            const definition = relationships.definitions.find(
              (item) => item.id === relationship.definitionId,
            );
            return (
              <p key={relationship.id}>
                {source ? definition?.forwardLabel : definition?.inverseLabel}{" "}
                {onPreview && other.id ? (
                  <button className="text-link" onClick={() => onPreview(other.id!)}>
                    {other.label}
                  </button>
                ) : (
                  <strong>{other.label}</strong>
                )}
              </p>
            );
          })}
          {connections.length > 8 && (
            <p className="muted">Open the Entry to see all {connections.length} connections.</p>
          )}
        </section>
      )}
    </div>
  );
}

export function EntryContextPanel({
  projectId,
  entryId,
  categories,
  opener,
  onClose,
  onPreview,
  onOpen,
  openDisabled,
}: {
  projectId: string;
  entryId: string;
  categories: Category[];
  opener: HTMLElement | null;
  onClose: () => void;
  onPreview: (id: string) => void;
  onOpen: (id: string) => void;
  openDisabled: boolean;
}) {
  const [entry, setEntry] = useState<Entry | null>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const close = useRef(onClose);
  close.current = onClose;
  function dismiss() {
    close.current();
    if (opener?.isConnected) opener.focus({ preventScroll: true });
  }
  useEffect(() => {
    closeButton.current?.focus({ preventScroll: true });
    function keydown(event: KeyboardEvent) {
      if (event.key === "Escape" && !document.querySelector("dialog[open]")) {
        event.preventDefault();
        close.current();
        if (opener?.isConnected) opener.focus({ preventScroll: true });
      }
    }
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [opener]);
  return (
    <aside className="entry-context-panel" aria-label="Entry reference">
      <header>
        <div>
          <small className="muted">Beside your writing · saved reference</small>
          <h2>{entry?.displayName ?? "Entry reference"}</h2>
        </div>
        <button
          ref={closeButton}
          className="quiet-button"
          aria-label="Close reference"
          onClick={dismiss}
        >
          ×
        </button>
      </header>
      <div className="entry-context-scroll">
        <EntryContextContent
          projectId={projectId}
          entryId={entryId}
          categories={categories}
          onPreview={onPreview}
          onLoaded={setEntry}
        />
      </div>
      <footer>
        <button disabled={openDisabled || !entry} onClick={() => onOpen(entryId)}>
          Open Entry for editing
        </button>
        <p className="muted">Your current writing is saved before switching.</p>
      </footer>
    </aside>
  );
}
