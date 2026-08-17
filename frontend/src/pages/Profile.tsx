import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router";

import {
  Button,
  Card,
  Field,
  FormFeedback,
  PageTitle,
  SectionTitle,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, clearToken } from "../lib/api";
import { translateApiError } from "../lib/errors";
import { useMe } from "../lib/session";

export default function Profile() {
  const { data: me } = useMe();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  if (!me) return null;

  const savePassword = async () => {
    if (password.length < 8) {
      setSuccess(null);
      setError(t.profile.passwordTooShort);
      return;
    }
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await api.setMyPassword(password);
      setSuccess(t.profile.passwordSaved);
      setPassword("");
      // Refresh has_password so the section flips to "change password".
      queryClient.invalidateQueries({ queryKey: ["me"] });
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  const logout = async (everywhere: boolean) => {
    try {
      if (everywhere) {
        await api.logoutAll();
      } else {
        await api.logout();
      }
    } finally {
      clearToken();
      queryClient.clear();
      navigate("/login", { replace: true });
    }
  };

  return (
    <>
      <PageTitle>{t.profile.title}</PageTitle>
      <Card>
        <div className="font-bold">
          {me.user.first_name} {me.user.last_name}
        </div>
        <div className="text-sm text-ink-soft">{me.user.email}</div>
      </Card>

      <SectionTitle>
        {me.has_password ? t.profile.changePassword : t.profile.setPassword}
      </SectionTitle>
      <Card>
        <Field
          label={me.has_password ? t.profile.newPassword : t.common.password}
          hint={
            me.has_password ? t.profile.changePasswordHint : t.profile.passwordHint
          }
        >
          <TextInput
            type="password"
            autoComplete="new-password"
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </Field>
        <Button
          className="mt-3"
          disabled={busy || password.length === 0}
          onClick={savePassword}
        >
          {t.common.save}
        </Button>
        <FormFeedback error={error} success={success} />
      </Card>

      <div className="mt-8 flex flex-col gap-3">
        <Button variant="secondary" onClick={() => logout(false)}>
          {t.profile.logout}
        </Button>
        <Button variant="danger" onClick={() => logout(true)}>
          {t.profile.logoutAll}
        </Button>
      </div>
    </>
  );
}
