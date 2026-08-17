// Small shared UI primitives — mobile-first, 44px tap targets, Abysse tokens.
import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
} from "react";

import { t } from "../i18n/fr";

type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

const buttonStyles: Record<ButtonVariant, string> = {
  primary:
    "bg-abysse-600 text-white hover:bg-abysse-700 disabled:bg-abysse-200",
  secondary:
    "bg-white text-abysse-700 border border-hairline hover:bg-abysse-50",
  danger: "bg-danger-50 text-danger-600 border border-danger-600/20 hover:bg-danger-600 hover:text-white",
  ghost: "text-abysse-700 hover:bg-abysse-50",
};

export function Button({
  variant = "primary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return (
    <button
      {...props}
      className={`inline-flex min-h-11 items-center justify-center gap-2 rounded-(--radius-control) px-4 text-[15px] font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-60 ${buttonStyles[variant]} ${className}`}
    />
  );
}

export function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-semibold text-ink-soft">
        {label}
      </span>
      {children}
      {hint && <span className="mt-1 block text-xs text-ink-soft">{hint}</span>}
    </label>
  );
}

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`min-h-11 w-full rounded-(--radius-control) border border-hairline bg-white px-3 text-[16px] outline-none focus:border-abysse-600 focus:ring-2 focus:ring-abysse-100 ${props.className ?? ""}`}
    />
  );
}

export function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={`min-h-11 rounded-(--radius-control) border border-hairline bg-white px-3 text-[16px] outline-none focus:border-abysse-600 ${props.className ?? ""}`}
    />
  );
}

/** Prev/next pagination controls; hidden when everything fits on one page. */
export function Pager({
  page,
  perPage,
  total,
  onPage,
}: {
  page: number;
  perPage: number;
  total: number;
  onPage: (page: number) => void;
}) {
  const pages = Math.max(1, Math.ceil(total / perPage));
  if (pages <= 1) return null;
  return (
    <div className="flex items-center justify-between gap-2 py-1">
      <Button
        variant="secondary"
        disabled={page <= 1}
        onClick={() => onPage(page - 1)}
      >
        ← {t.common.previous}
      </Button>
      <span className="text-sm text-ink-soft">{t.common.pageOf(page, pages)}</span>
      <Button
        variant="secondary"
        disabled={page >= pages}
        onClick={() => onPage(page + 1)}
      >
        {t.common.next} →
      </Button>
    </div>
  );
}

export function Card({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`rounded-(--radius-card) border border-hairline bg-white p-4 shadow-[0_1px_2px_rgba(20,37,46,0.06)] ${className}`}
    >
      {children}
    </div>
  );
}

export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "primary" | "success" | "warning" | "danger";
}) {
  const tones = {
    neutral: "bg-mist text-ink-soft",
    primary: "bg-abysse-50 text-abysse-700",
    success: "bg-success-50 text-success-600",
    warning: "bg-warning-50 text-warning-600",
    danger: "bg-danger-50 text-danger-600",
  };
  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold ${tones[tone]}`}
    >
      {children}
    </span>
  );
}

export function PageTitle({ children }: { children: ReactNode }) {
  return <h1 className="mb-4 text-xl font-bold">{children}</h1>;
}

export function SectionTitle({ children }: { children: ReactNode }) {
  return <h2 className="mb-2 mt-6 text-base font-bold">{children}</h2>;
}

export function Loading() {
  return (
    <div className="py-10 text-center text-sm text-ink-soft" role="status">
      {t.common.loading}
    </div>
  );
}

export function ErrorState({
  message,
  onRetry,
}: {
  message?: string;
  onRetry?: () => void;
}) {
  return (
    <Card className="border-danger-600/30 bg-danger-50 text-center">
      <p className="text-sm text-danger-600">{message ?? t.common.error}</p>
      {onRetry && (
        <Button variant="secondary" className="mt-3" onClick={onRetry}>
          {t.common.retry}
        </Button>
      )}
    </Card>
  );
}

export function EmptyState({ message }: { message?: string }) {
  return (
    <Card className="text-center text-sm text-ink-soft">
      {message ?? t.common.emptyList}
    </Card>
  );
}

/** Inline feedback under a form after a mutation. */
export function FormFeedback({
  error,
  success,
}: {
  error?: string | null;
  success?: string | null;
}) {
  if (error) {
    return <p className="mt-2 text-sm font-medium text-danger-600">{error}</p>;
  }
  if (success) {
    return (
      <p className="mt-2 text-sm font-medium text-success-600">{success}</p>
    );
  }
  return null;
}
