import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { Card, Loading } from "../components/ui";
import { t } from "../i18n/fr";
import { api, setToken } from "../lib/api";

// The token is strictly single-use: guard at module scope so even React
// StrictMode's dev-only unmount/remount cannot consume it twice.
const consumedTokens = new Set<string>();

export default function MagicLink() {
  const { token } = useParams<{ token: string }>();
  const [failed, setFailed] = useState(false);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!token || consumedTokens.has(token)) return;
    consumedTokens.add(token);
    api
      .consumeMagicLink(token)
      .then((session) => {
        setToken(session.token);
        // Principal changed: drop every cached response.
        queryClient.clear();
        navigate("/", { replace: true });
      })
      .catch(() => setFailed(true));
  }, [token, navigate, queryClient]);

  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <Card className="text-center">
        {failed ? (
          <>
            <p className="text-sm text-danger-600">{t.magicLink.invalid}</p>
            <Link
              to="/login"
              className="mt-4 inline-block font-semibold text-abysse-700"
            >
              {t.magicLink.backToLogin}
            </Link>
          </>
        ) : (
          <>
            <p className="font-semibold">{t.magicLink.title}</p>
            <Loading />
          </>
        )}
      </Card>
    </main>
  );
}
