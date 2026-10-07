import { useEffect, useRef, useState } from "react";
import { Dialog } from "./Dialog";
import { RichTextEditor, type WritingPosition } from "./RichTextEditor";
import { plainDocument } from "./chapterRecovery";
import type { FieldsController } from "./EntryFieldsPanel";
import type { useEntryDocument } from "./useEntryDocument";
import "./EntryDescriptionPanel.css";

const EMPTY_DESCRIPTION = { type: "doc", content: [{ type: "paragraph" }] };

export function EntryDocumentPanel({
  description,
  label = "Entry description",
  noun = "description",
  alwaysOpen = false,
  disabled = false,
  onController,
}: {
  description: ReturnType<typeof useEntryDocument>;
  label?: string;
  noun?: string;
  alwaysOpen?: boolean;
  disabled?: boolean;
  onController: (controller: FieldsController) => void;
}) {
  const [started, setStarted] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const position = useRef<WritingPosition | undefined>(undefined);
  const { flush, waitForPending } = description;
  useEffect(() => {
    onController({
      state: description.state,
      canSubmit: !description.recovery,
      submit: flush,
      waitForPending,
    });
  }, [description.state, description.recovery, onController, flush, waitForPending]);

  const WritingContainer = alwaysOpen ? "div" : "details";
  const document = description.snapshot?.document;
  const readOnly = description.snapshot?.workspaceState !== "active";
  const showWriting =
    !!description.snapshot && (alwaysOpen || started || !!document || !!description.draft);
  const recovery = description.recovery;
  const canRestore = !readOnly && !document?.readOnlyReason && !recovery?.readOnlyReason;
  return (
    <section className="entry-description" aria-label={label}>
      {!description.snapshot && !description.error && <p role="status">Loading {noun}…</p>}
      {description.error && (
        <div role="alert" className="writing-error">
          <strong>
            {description.draft
              ? `${noun === "description" ? "Description" : label} changes are not being saved.`
              : `${noun === "description" ? "Description" : label} unavailable.`}
          </strong>
          <p>
            {description.error}
            {description.draft ? " Your writing is kept here." : ""}
          </p>
          {description.draft && (
            <button disabled={description.reviewing} onClick={() => void description.retry()}>
              Retry saving {noun}
            </button>
          )}
          <button disabled={description.reviewing} onClick={() => void description.reviewSaved()}>
            {description.snapshot ? `Review saved ${noun}` : `Retry loading ${noun}`}
          </button>
          {description.draft && (
            <details>
              <summary>Copy unsaved {noun}</summary>
              <textarea
                readOnly
                aria-label={`Unsaved ${noun}`}
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
          An unsaved {noun} needs review.{" "}
          <button className="quiet-button" onClick={() => setRecoveryOpen(true)}>
            Review {noun} draft
          </button>
        </p>
      )}
      {description.snapshot && !showWriting && !recovery && (
        <button
          className="quiet-button"
          disabled={disabled || readOnly}
          onClick={() => setStarted(true)}
        >
          Add {noun}
        </button>
      )}
      {showWriting && (
        <WritingContainer className="entry-description-writing" open={!alwaysOpen || undefined}>
          {!alwaysOpen && <summary>{noun === "description" ? "Description" : label}</summary>}
          {document?.readOnlyReason ? (
            <div role="alert">
              <p>{document.readOnlyReason} The saved original has not been changed.</p>
              <p className="description-preserved-text">{document.plainText}</p>
              <details>
                <summary>Original {noun}</summary>
                <textarea
                  aria-label={`Original ${noun} data`}
                  readOnly
                  value={document.originalJson ?? ""}
                />
              </details>
            </div>
          ) : !recovery ? (
            <RichTextEditor
              key={description.editorEpoch}
              label={label}
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
        </WritingContainer>
      )}
      <Dialog
        open={recoveryOpen && !!recovery}
        title={`Review ${noun} draft`}
        onClose={() => setRecoveryOpen(false)}
      >
        {recovery && (
          <>
            <p>Compare the draft with the saved {noun} before choosing which to keep.</p>
            {recovery.readOnlyReason && (
              <div role="alert">
                <p>{recovery.readOnlyReason}</p>
                <textarea
                  aria-label={`Original ${noun} recovery data`}
                  readOnly
                  value={recovery.originalRaw ?? ""}
                />
              </div>
            )}
            <div className="recovery-comparison">
              <label>
                Saved {noun}
                <textarea readOnly value={document?.plainText ?? ""} />
              </label>
              <label>
                Recovered {noun}
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
                Use {noun} draft
              </button>
              <button
                onClick={() => {
                  description.keepSaved();
                  setRecoveryOpen(false);
                }}
              >
                Keep saved {noun}
              </button>
            </div>
            {!canRestore && (
              <p>Copy the draft above. Unsupported or inactive writing cannot be overwritten.</p>
            )}
          </>
        )}
      </Dialog>
    </section>
  );
}
