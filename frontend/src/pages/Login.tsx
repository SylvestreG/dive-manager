import { useQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { Link, useSearchParams } from "react-router";

import {
  Button,
  Card,
  Field,
  FormFeedback,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, oidcStartUrl } from "../lib/api";
import { translateApiError } from "../lib/errors";

/**
 * Login (ADR-0009, no passwords): SSO buttons for the providers configured
 * on this instance, then the magic link. The magic-link response is generic
 * whether or not an account exists — no enumeration. `?email=` pre-fills
 * the address (welcome emails link here); `?error=` comes from the SSO
 * callback.
 */
export default function Login() {
  const [searchParams] = useSearchParams();
  const [email, setEmail] = useState(searchParams.get("email") ?? "");
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(() => {
    const code = searchParams.get("error");
    return code ? (t.login.errors[code] ?? t.login.errors.sso) : null;
  });
  const providers = useQuery({
    queryKey: ["auth", "providers"],
    queryFn: api.providers,
    staleTime: Infinity,
  });

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

  const ssoButtons = providers.data ?? [];

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
          <div className="flex flex-col gap-4">
            {ssoButtons.length > 0 && (
              <>
                <div className="flex flex-col gap-2">
                  {ssoButtons.map((provider) => (
                    // A full navigation: the backend redirects to the provider.
                    <a
                      key={provider.id}
                      href={oidcStartUrl(provider.id)}
                      className="inline-flex min-h-11 items-center justify-center rounded-(--radius-control) border border-hairline bg-white px-4 text-[15px] font-semibold text-ink hover:bg-abysse-50"
                    >
                      {t.login.continueWith(provider.label)}
                    </a>
                  ))}
                </div>
                <div className="flex items-center gap-3 text-xs text-ink-soft">
                  <span className="h-px flex-1 bg-hairline" />
                  {t.login.or}
                  <span className="h-px flex-1 bg-hairline" />
                </div>
              </>
            )}
            <form onSubmit={submit} className="flex flex-col gap-4">
              <Field label={t.common.email}>
                <TextInput
                  type="email"
                  autoComplete="email"
                  inputMode="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </Field>
              <Button type="submit" disabled={busy} className="w-full">
                {ssoButtons.length > 0 ? t.login.emailOption : t.login.sendLink}
              </Button>
            </form>
            <FormFeedback error={error} />
          </div>
        )}
      </Card>
      <Link
        to="/confidentialite"
        className="mt-6 text-center text-xs text-ink-soft underline"
      >
        {t.login.privacy}
      </Link>
    </main>
  );
}
