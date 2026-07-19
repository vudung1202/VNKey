import { useEffect, useRef } from "react";

import { Button } from "./Button";

/**
 * In-app replacement for the native `confirm()` dialog. Controlled via `open`.
 *
 * Three exits, deliberately distinct: the confirm button (`onConfirm`), the
 * explicit cancel button (`onCancel`), and dismissal via Esc or a backdrop
 * click (`onDismiss`, falling back to `onCancel`). Splitting dismissal out
 * matters when "cancel" is itself a real, possibly destructive choice (e.g.
 * macro import's replace): a stray backdrop click should abort, not run it.
 * Focus is trapped to the two buttons and lands on confirm.
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
  onDismiss,
}: {
  open: boolean;
  title?: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  onDismiss?: () => void;
}) {
  const modalRef = useRef<HTMLDivElement>(null);
  const dismiss = onDismiss ?? onCancel;

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        dismiss();
        return;
      }
      if (e.key === "Tab") {
        const focusables = modalRef.current?.querySelectorAll<HTMLElement>("button:not([disabled])");
        if (!focusables || focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        const active = document.activeElement;
        if (active && !modalRef.current?.contains(active)) {
          e.preventDefault();
          first.focus();
        } else if (e.shiftKey && active === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, dismiss]);

  if (!open) return null;

  return (
    <div className="ui-modal-backdrop" onMouseDown={dismiss}>
      <div
        ref={modalRef}
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
