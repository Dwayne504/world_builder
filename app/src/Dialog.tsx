import { useEffect, useId, useRef, type ReactNode } from "react";

/** Native modality provides focus containment, Escape and focus restoration.
 * Children stay mounted when closed so hiding a dialog never discards a draft.
 */
export function Dialog({
  open,
  title,
  onClose,
  children,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);
  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header className="dialog-header">
        <h2 id={titleId}>{title}</h2>
        <button className="quiet-button" aria-label={`Close ${title}`} onClick={onClose}>
          Done
        </button>
      </header>
      <div className="dialog-body">{children}</div>
    </dialog>
  );
}
