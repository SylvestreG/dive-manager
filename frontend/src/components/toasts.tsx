// Global error toasts — the safety net behind the frontend rule "every error
// is visibly surfaced". Fed automatically by the react-query MutationCache
// (see main.tsx); pages never need to remember to display mutation errors.
import { useSyncExternalStore } from "react";

export interface Toast {
  id: number;
  message: string;
}

let toasts: Toast[] = [];
let nextId = 1;
const listeners = new Set<() => void>();

function emit() {
  toasts = [...toasts];
  listeners.forEach((listener) => listener());
}

export function pushErrorToast(message: string) {
  const id = nextId++;
  toasts.push({ id, message });
  emit();
  setTimeout(() => {
    toasts = toasts.filter((toast) => toast.id !== id);
    emit();
  }, 6000);
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

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
          className="pointer-events-auto w-full max-w-sm rounded-(--radius-card) border border-danger-600/30 bg-danger-50 px-4 py-3 text-sm font-semibold text-danger-600 shadow-lg"
        >
          {toast.message}
        </div>
      ))}
    </div>
  );
}
