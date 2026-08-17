import { useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router";

import {
  Button,
  Card,
  Field,
  FormFeedback,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, ApiError, setToken } from "../lib/api";
import { translateApiError } from "../lib/errors";

/**
 * Email-first login (product rule): the magic link is only offered to
 * accounts without a password. "Continuer" either sends the link right away
 * (password-less or unknown account — indistinguishable) or reveals the
 * password field, with a forgot-password fallback that sends a link.
 */
export default function Login() {
  const [step, setStep] = useState<"email" | "password" | "link-sent">("email");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const sendMagicLink = async () => {
    await api.requestMagicLink(email);
    setStep("link-sent");
  };

  const continueWithEmail = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const methods = await api.loginMethods(email);
      if (methods.password) {
        setStep("password");
      } else {
        await sendMagicLink();
      }
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  const submitPassword = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const session = await api.loginPassword(email, password);
      setToken(session.token);
      queryClient.clear();
      navigate("/", { replace: true });
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        setError(t.login.invalidCredentials);
      } else {
        setError(translateApiError(e));
      }
      setBusy(false);
    }
  };

  const forgotPassword = async () => {
    setBusy(true);
    setError(null);
    try {
      await sendMagicLink();
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  const backToEmail = () => {
    setStep("email");
    setPassword("");
    setError(null);
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <div className="mb-6 text-center">
        <div className="text-2xl font-bold text-abysse-700">{t.appName}</div>
        <p className="mt-1 text-sm text-ink-soft">{t.login.intro}</p>
      </div>
      <Card>
        {step === "email" && (
          <form onSubmit={continueWithEmail} className="flex flex-col gap-4">
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
              {t.login.continue}
            </Button>
            <FormFeedback error={error} />
          </form>
        )}

        {step === "password" && (
          <form onSubmit={submitPassword} className="flex flex-col gap-4">
            <div className="text-sm text-ink-soft">{email}</div>
            <Field label={t.common.password}>
              <TextInput
                type="password"
                autoComplete="current-password"
                required
                autoFocus
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </Field>
            <Button type="submit" disabled={busy} className="w-full">
              {t.login.passwordAction}
            </Button>
            <FormFeedback error={error} />
            <button
              type="button"
              className="min-h-11 text-sm font-semibold text-abysse-700"
              disabled={busy}
              onClick={forgotPassword}
            >
              {t.login.forgotPassword}
            </button>
          </form>
        )}

        {step === "link-sent" && (
          <div className="flex flex-col gap-4 text-center">
            <p className="text-sm font-medium text-success-600">
              {t.login.magicLinkSent}
            </p>
          </div>
        )}

        {step !== "email" && (
          <button
            className="mt-4 w-full text-center text-sm font-semibold text-ink-soft"
            onClick={backToEmail}
          >
            {t.login.changeEmail}
          </button>
        )}
      </Card>
    </main>
  );
}
