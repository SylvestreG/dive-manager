import { useState, type FormEvent } from "react";
import { useSearchParams } from "react-router";

import {
  Button,
  Card,
  Field,
  FormFeedback,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api } from "../lib/api";
import { translateApiError } from "../lib/errors";

/**
 * Login by magic link (ADR-0009: no passwords). The response is generic
 * whether or not an account exists — no enumeration. `?email=` pre-fills the
 * address (welcome emails link here).
 */
export default function Login() {
  const [searchParams] = useSearchParams();
  const [email, setEmail] = useState(searchParams.get("email") ?? "");
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.requestMagicLink(email);
      setSent(true);
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <div className="mb-6 text-center">
        <div className="text-2xl font-bold text-abysse-700">{t.appName}</div>
        <p className="mt-1 text-sm text-ink-soft">{t.login.intro}</p>
      </div>
      <Card>
        {sent ? (
          <div className="flex flex-col gap-4 text-center">
            <p className="text-sm font-medium text-success-600">
              {t.login.magicLinkSent}
            </p>
            <button
              className="min-h-11 text-sm font-semibold text-ink-soft"
              onClick={() => setSent(false)}
            >
              {t.login.changeEmail}
            </button>
          </div>
        ) : (
          <form onSubmit={submit} className="flex flex-col gap-4">
            <Field label={t.common.email}>
              <TextInput
                type="email"
                autoComplete="email"
                inputMode="email"
                required
                autoFocus
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
            </Field>
            <Button type="submit" disabled={busy} className="w-full">
              {t.login.sendLink}
            </Button>
            <FormFeedback error={error} />
          </form>
        )}
      </Card>
    </main>
  );
}
