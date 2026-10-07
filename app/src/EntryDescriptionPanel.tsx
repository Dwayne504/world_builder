import { useEffect, useRef, useState } from "react";
import { Dialog } from "./Dialog";
import { RichTextEditor, type WritingPosition } from "./RichTextEditor";
import { plainDocument } from "./chapterRecovery";
import type { FieldsController } from "./EntryFieldsPanel";
import { useEntryDescription } from "./useEntryDescription";
import "./EntryDescriptionPanel.css";

const EMPTY_DESCRIPTION = { type: "doc", content: [{ type: "paragraph" }] };

export function EntryDescriptionPanel({
  projectId,
  entryId,
  disabled = false,
  onController,
  onRevision,
  getRevision,
}: {
  projectId: string;
  entryId: string;
  disabled?: boolean;
  onController: (controller: FieldsController) => void;
  onRevision: (revision: number) => void;
  getRevision: () => number;
}) {
  const description = useEntryDescription(projectId, entryId, onRevision, getRevision);
  const [started, setStarted] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const position = useRef<WritingPosition | undefined>(undefined);
  const { flush } = description;
  useEffect(() => {
    onController({
      state: description.state,
      canSubmit: !description.recovery,
      submit: flush,
    });
  }, [description.state, description.recovery, onController, flush]);

  const document = description.snapshot?.document;
  const readOnly = description.snapshot?.workspaceState !== "active";
  const showWriting = started || !!document || !!description.draft;
  const recovery = description.recovery;
  const canRestore = !readOnly && !document?.readOnlyReason && !recovery?.readOnlyReason;
  return (
    <section className="entry-description" aria-label="Entry description">
      {!description.snapshot && !description.error && <p role="status">Loading description…</p>}
      {description.error && (
        <div role="alert" className="writing-error">
          <strong>
            {description.draft
              ? "Description changes are not being saved."
              : "Description unavailable."}
          </strong>
          <p>
            {description.error}
            {description.draft ? " Your writing is kept here." : ""}
          </p>
          {description.draft && (
            <button disabled={description.reviewing} onClick={() => void description.retry()}>
              Retry saving description
            </button>
          )}
          <button disabled={description.reviewing} onClick={() => void description.reviewSaved()}>
            {description.snapshot ? "Review saved description" : "Retry loading description"}
          </button>
          {description.draft && (
            <details>
              <summary>Copy unsaved description</summary>
              <textarea
                readOnly
                aria-label="Unsaved description"
                value={plainDocument(description.draft)}
              />
            </details>
          )}
        </div>
      )}
      {!description.recoveryAvailable && (
        <p role="alert">
          The local recovery copy is unavailable. Keep this window open if saving fails.
        </p>
      )}
      {recovery && (
        <p role="status">
          An unsaved description needs review.{" "}
          <button className="quiet-button" onClick={() => setRecoveryOpen(true)}>
            Review description draft
          </button>
        </p>
      )}
      {description.snapshot && !showWriting && !recovery && (
        <button
          className="quiet-button"
          disabled={disabled || readOnly}
          onClick={() => setStarted(true)}
        >
          Add description
        </button>
      )}
      {showWriting && (
        <details className="entry-description-writing" open>
          <summary>Description</summary>
          {document?.readOnlyReason ? (
            <div role="alert">
              <p>{document.readOnlyReason} The saved original has not been changed.</p>
              <p className="description-preserved-text">{document.plainText}</p>
              <details>
                <summary>Original description</summary>
                <textarea
                  aria-label="Original description data"
                  readOnly
                  value={document.originalJson ?? ""}
                />
              </details>
            </div>
          ) : !recovery ? (
            <RichTextEditor
              key={description.editorEpoch}
              label="Entry description"
              locked={disabled || readOnly || description.reviewing}
              content={description.draft ?? document?.content ?? EMPTY_DESCRIPTION}
              onChange={description.change}
              onBlur={() => {
                if (description.state !== "failed") void flush();
              }}
              position={position.current}
              onPosition={(next) => {
                position.current = next;
              }}
            />
          ) : null}
        </details>
      )}
      <Dialog
        open={recoveryOpen && !!recovery}
        title="Review description draft"
        onClose={() => setRecoveryOpen(false)}
      >
        {recovery && (
          <>
            <p>Compare the draft with the saved description before choosing which to keep.</p>
            {recovery.readOnlyReason && (
              <div role="alert">
                <p>{recovery.readOnlyReason}</p>
                <textarea
                  aria-label="Original description recovery data"
                  readOnly
                  value={recovery.originalRaw ?? ""}
                />
              </div>
            )}
            <div className="recovery-comparison">
              <label>
                Saved description
                <textarea readOnly value={document?.plainText ?? ""} />
              </label>
              <label>
                Recovered description
                <textarea
                  readOnly
                  value={recovery.content ? plainDocument(recovery.content) : ""}
                />
              </label>
            </div>
            <div className="row">
              <button
                disabled={!canRestore}
                onClick={() => {
                  description.useRecovered();
                  setRecoveryOpen(false);
                }}
              >
                Use description draft
              </button>
              <button
                onClick={() => {
                  description.keepSaved();
                  setRecoveryOpen(false);
                }}
              >
                Keep saved description
              </button>
            </div>
            {!canRestore && (
              <p>
                Copy the draft above. An unsupported or inactive description cannot be overwritten.
              </p>
            )}
          </>
        )}
      </Dialog>
    </section>
  );
}
