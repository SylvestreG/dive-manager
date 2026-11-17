import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useSearchParams } from "react-router";

import { t } from "../i18n/fr";
import { api, type ProviderId } from "../lib/api";
import { translateApiError } from "../lib/errors";
import { Button, Card, Field, FormFeedback, TextInput } from "./ui";

const PROVIDER_LABEL: Record<ProviderId, string> = {
  google: "Google",
  microsoft: "Microsoft",
  apple: "Apple",
};

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

/** Profile: email (magic link) + linked SSO identities (ADR-0009 R10–R12). */
export function LoginMethodsCard() {
  const queryClient = useQueryClient();
  const [params, setParams] = useSearchParams();
  const linked = params.get("linked");
  const linkError = params.get("link_error");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(
    linked ? t.loginMethods.linked(PROVIDER_LABEL[linked as ProviderId] ?? linked) : null,
  );
  const [newEmail, setNewEmail] = useState("");
  const methods = useQuery({ queryKey: ["me", "login-methods"], queryFn: api.loginMethods });
  const m = t.loginMethods;

  const clearParams = () => {
    if (linked || linkError) setParams({}, { replace: true });
  };
  const link = useMutation({
    meta: { inlineError: true },
    mutationFn: (provider: ProviderId) => api.startLink(provider),
    // Full navigation to the provider; it comes back to /profile?linked=…
    onSuccess: ({ url }) => window.location.assign(url),
    onError: (e) => setError(translateApiError(e)),
  });
  const unlink = useMutation({
    meta: { inlineError: true },
    mutationFn: (id: string) => api.unlinkIdentity(id),
    onSuccess: () => {
      clearParams();
      setError(null);
      setSuccess(m.removed);
      queryClient.invalidateQueries({ queryKey: ["me", "login-methods"] });
    },
    onError: (e) => {
      setSuccess(null);
      setError(translateApiError(e));
    },
  });
  const changeEmail = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.changeEmail(newEmail),
    onSuccess: () => {
      setError(null);
      setSuccess(m.emailConfirmationSent(newEmail.trim()));
      setNewEmail("");
    },
    onError: (e) => {
      setSuccess(null);
      setError(translateApiError(e));
    },
  });

  if (!methods.data) return null;
  const data = methods.data;
  const linkedProviders = new Set(data.identities.map((i) => i.provider));
  const linkable = data.available_providers.filter((p) => !linkedProviders.has(p));

  return (
    <Card>
      <ul className="flex flex-col gap-3 text-sm">
        <li className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="font-semibold">{m.emailLink}</div>
            <div className="truncate text-ink-soft">{data.email ?? m.noEmail}</div>
          </div>
        </li>
        {data.identities.map((identity) => (
          <li key={identity.id} className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <div className="font-semibold">{PROVIDER_LABEL[identity.provider]}</div>
              <div className="truncate text-ink-soft">
                {identity.email ?? ""}
                {identity.last_used_at && ` · ${m.lastUsed(formatDate(identity.last_used_at))}`}
              </div>
            </div>
            <Button
              variant="ghost"
              disabled={unlink.isPending}
              onClick={() => unlink.mutate(identity.id)}
            >
              {m.remove}
            </Button>
          </li>
        ))}
      </ul>
      {linkable.length > 0 && (
        <div className="mt-4 flex flex-col gap-2">
          {linkable.map((provider) => (
            <Button
              key={provider}
              variant="secondary"
              disabled={link.isPending}
              onClick={() => link.mutate(provider)}
            >
              {m.add(PROVIDER_LABEL[provider])}
            </Button>
          ))}
        </div>
      )}
      <div className="mt-4 flex flex-col gap-2 border-t border-hairline pt-4">
        <Field label={data.email ? m.changeEmail : m.setEmail}>
          <TextInput
            type="email"
            inputMode="email"
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
          />
        </Field>
        <Button
          variant="secondary"
          disabled={!newEmail.trim() || changeEmail.isPending}
          onClick={() => changeEmail.mutate()}
        >
          {m.sendConfirmation}
        </Button>
      </div>
      <FormFeedback
        error={error ?? (linkError ? m.alreadyUsed : null)}
        success={success}
      />
    </Card>
  );
}

/** Profile: my live sessions (R29), each revocable. */
export function DevicesCard() {
  const queryClient = useQueryClient();
  const devices = useQuery({ queryKey: ["me", "sessions"], queryFn: api.devices });
  const revoke = useMutation({
    mutationFn: (id: string) => api.revokeDevice(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["me", "sessions"] }),
  });
  const m = t.loginMethods;
  if (!devices.data) return null;
  return (
    <Card>
      <ul className="flex flex-col gap-3 text-sm">
        {devices.data.map((device) => (
          <li key={device.id} className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <div className="font-semibold">
                {device.label ?? m.unknownDevice}
                {device.current && (
                  <span className="ml-2 text-xs font-normal text-success-600">
                    {m.thisDevice}
                  </span>
                )}
              </div>
              <div className="text-ink-soft">{m.lastSeen(formatDate(device.last_seen_at))}</div>
            </div>
            {!device.current && (
              <Button
                variant="ghost"
                disabled={revoke.isPending}
                onClick={() => revoke.mutate(device.id)}
              >
                {m.logoutDevice}
              </Button>
            )}
          </li>
        ))}
      </ul>
    </Card>
  );
}
