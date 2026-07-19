import { useEffect } from "react";

import { Button } from "./Button";

/**
 * In-app replacement for the native `confirm()` dialog. Controlled via `open`;
 * Esc and backdrop click both cancel. Focus lands on the confirm button so
 * keyboard users can Enter/Escape immediately.
 */
export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  danger,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title?: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onCancel();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onCancel]);

  if (!open) return null;

  return (
    <div className="ui-modal-backdrop" onMouseDown={onCancel}>
      <div
        className="ui-modal"
        role="dialog"
        aria-modal="true"
        aria-label={title ?? message}
        onMouseDown={(e) => e.stopPropagation()}
      >
        {title && <div className="ui-modal__title">{title}</div>}
        <div className="ui-modal__body">{message}</div>
        <div className="ui-modal__actions">
          <Button variant="ghost" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button variant={danger ? "danger" : "primary"} autoFocus onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
