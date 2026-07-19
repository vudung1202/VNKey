import { useEffect, useState } from "react";

/**
 * Tiny toast store + host. `showToast` can be called from anywhere (event
 * handlers, IPC callbacks) without threading a context; each window that wants
 * toasts renders one `<ToastHost/>`. Replaces the native `message()` dialog
 * for transient success/failure notices.
 */
export type ToastKind = "success" | "error";
type ToastItem = { id: number; title?: string; sub?: string; kind: ToastKind };

const listeners = new Set<(items: ToastItem[]) => void>();
let items: ToastItem[] = [];
let seq = 0;

function emit() {
  for (const listener of listeners) listener(items);
}

export function showToast(toast: { title?: string; sub?: string; kind?: ToastKind }, durationMs = 3200) {
  const id = ++seq;
  items = [...items, { id, title: toast.title, sub: toast.sub, kind: toast.kind ?? "success" }];
  emit();
  window.setTimeout(() => {
    items = items.filter((item) => item.id !== id);
    emit();
  }, durationMs);
}

export function ToastHost() {
  const [list, setList] = useState<ToastItem[]>(items);

  useEffect(() => {
    listeners.add(setList);
    setList(items);
    return () => {
      listeners.delete(setList);
    };
  }, []);

  return (
    <div className="ui-toast-host" role="status" aria-live="polite">
      {list.map((toast) => (
        <div key={toast.id} className={`ui-toast${toast.kind === "error" ? " ui-toast--error" : ""}`}>
          <span className="ui-toast__dot" />
          <div className="ui-toast__body">
            {toast.title && <span className="ui-toast__title">{toast.title}</span>}
            {toast.sub && <span className="ui-toast__sub">{toast.sub}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}
