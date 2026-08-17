import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import {
  Badge,
  Button,
  Card,
  Field,
  FormFeedback,
  Loading,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, setToken } from "../lib/api";
import { translateApiError } from "../lib/errors";

export default function AcceptInvitation() {
  const { token } = useParams<{ token: string }>();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const preview = useQuery({
    queryKey: ["invitation", token],
    queryFn: () => api.previewInvitation(token!),
    enabled: !!token,
    retry: false,
  });

  const accept = async () => {
    if (password && password.length < 8) {
      setError(t.profile.passwordTooShort);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const session = await api.acceptInvitation(token!, password || undefined);
      setToken(session.token);
      queryClient.clear();
      navigate("/", { replace: true });
    } catch (e) {
      setError(translateApiError(e));
      setBusy(false);
    }
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center px-4">
      <Card>
        <h1 className="mb-3 text-lg font-bold">{t.invitation.title}</h1>
        {preview.isPending && <Loading />}
        {preview.isError && (
          <>
            <p className="text-sm text-danger-600">{t.invitation.invalid}</p>
            <Link
              to="/login"
              className="mt-4 inline-block font-semibold text-abysse-700"
            >
              {t.magicLink.backToLogin}
            </Link>
          </>
        )}
        {preview.data && (
          <div className="flex flex-col gap-4">
            <p className="text-sm">
              {preview.data.club_name
                ? t.invitation.joinClub(preview.data.club_name)
                : t.invitation.joinPlatform}
            </p>
            <div className="flex flex-wrap items-center gap-2 text-sm">
              <span className="font-semibold">
                {preview.data.first_name} {preview.data.last_name}
              </span>
              <span className="text-ink-soft">{preview.data.email}</span>
              {preview.data.role_name && (
                <Badge tone="primary">{preview.data.role_name}</Badge>
              )}
            </div>
            <Field
              label={t.common.password}
              hint={t.invitation.optionalPassword}
            >
              <TextInput
                type="password"
                autoComplete="new-password"
                minLength={8}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </Field>
            <Button onClick={accept} disabled={busy} className="w-full">
              {t.invitation.accept}
            </Button>
            <FormFeedback error={error} />
          </div>
        )}
      </Card>
    </main>
  );
}
