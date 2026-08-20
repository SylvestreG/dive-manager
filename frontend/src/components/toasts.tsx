// Global toasts — the safety net behind the frontend rule "every error is
// visibly surfaced". Errors are fed automatically by the react-query
// MutationCache (see main.tsx); pages never need to remember to display
// mutation errors. Success toasts are opt-in (immediate-persistence UIs like
// the stamp-position editor, which have no form to anchor feedback to).
import { useSyncExternalStore } from "react";

export interface Toast {
  id: number;
  message: string;
  tone: "error" | "success";
}

let toasts: Toast[] = [];
let nextId = 1;
const listeners = new Set<() => void>();

function emit() {
  toasts = [...toasts];
  listeners.forEach((listener) => listener());
}

function push(message: string, tone: Toast["tone"], durationMs: number) {
  const id = nextId++;
  toasts.push({ id, message, tone });
  emit();
  setTimeout(() => {
    toasts = toasts.filter((toast) => toast.id !== id);
    emit();
  }, durationMs);
}

export function pushErrorToast(message: string) {
  push(message, "error", 6000);
}

export function pushToast(message: string) {
  push(message, "success", 2500);
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const TONE_STYLES: Record<Toast["tone"], string> = {
  error: "border-danger-600/30 bg-danger-50 text-danger-600",
  success: "border-success-600/30 bg-success-50 text-success-600",
};

export function ToastViewport() {
  const current = useSyncExternalStore(subscribe, () => toasts);
  if (current.length === 0) return null;
  return (
    <div
      className="pointer-events-none fixed inset-x-0 top-2 z-50 flex flex-col items-center gap-2 px-4"
      style={{ paddingTop: "env(safe-area-inset-top)" }}
    >
      {current.map((toast) => (
        <div
          key={toast.id}
          role="alert"
          className={`pointer-events-auto w-full max-w-sm rounded-(--radius-card) border px-4 py-3 text-sm font-semibold shadow-lg ${TONE_STYLES[toast.tone]}`}
        >
          {toast.message}
        </div>
      ))}
    </div>
  );
}
