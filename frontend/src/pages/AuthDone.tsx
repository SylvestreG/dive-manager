import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";

import { Card, Loading } from "../components/ui";
import { t } from "../i18n/fr";
import { api, setToken } from "../lib/api";

// One-time code: guard at module scope (StrictMode double effects).
const exchangedCodes = new Set<string>();

/** SSO landing page: trades the one-time code for a session (ADR-0009 —
 * the session token itself never appears in a URL). */
export default function AuthDone() {
  const [params] = useSearchParams();
  const code = params.get("code");
  const [failed, setFailed] = useState(!code);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!code || exchangedCodes.has(code)) return;
    exchangedCodes.add(code);
    // Drop the one-time code from the address bar / history right away.
    // Keep React Router's history state (idx/key) intact.
    window.history.replaceState(window.history.state, "", window.location.pathname);
    api
      .exchangeCode(code)
      .then((session) => {
        setToken(session.token);
        queryClient.clear();
        navigate("/", { replace: true });
      })
      .catch(() => setFailed(true));
  }, [code, navigate, queryClient]);

  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <Card className="text-center">
        {failed ? (
          <>
            <p className="text-sm text-danger-600">{t.sso.failed}</p>
            <Link to="/login" className="mt-4 inline-block font-semibold text-abysse-700">
              {t.magicLink.backToLogin}
            </Link>
          </>
        ) : (
          <>
            <p className="font-semibold">{t.sso.finishing}</p>
            <Loading />
          </>
        )}
      </Card>
    </main>
  );
}

/** Static outcome pages of the SSO callback. */
export function AuthNotice({ kind }: { kind: "check-email" | "unknown" }) {
  const title = kind === "check-email" ? t.sso.checkEmailTitle : t.sso.unknownTitle;
  const text = kind === "check-email" ? t.sso.checkEmail : t.sso.unknown;
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <Card className="text-center">
        <p className="font-semibold">{title}</p>
        <p className="mt-2 text-sm text-ink-soft">{text}</p>
        <Link to="/login" className="mt-4 inline-block font-semibold text-abysse-700">
          {t.magicLink.backToLogin}
        </Link>
      </Card>
    </main>
  );
}
