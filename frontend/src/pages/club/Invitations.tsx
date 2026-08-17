import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useState } from "react";

import {
  EMPTY_PERSON,
  PersonFields,
  personComplete,
} from "../../components/PersonFields";
import {
  Button,
  Card,
  EmptyState,
  ErrorState,
  Field,
  FormFeedback,
  Loading,
  Pager,
  SectionTitle,
  Select,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api } from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { useClub } from "./ClubLayout";

export default function Invitations() {
  const club = useClub();
  const queryClient = useQueryClient();
  const [page, setPage] = useState(1);

  const invitations = useQuery({
    queryKey: ["club", club.slug, "invitations", page],
    queryFn: () => api.club(club.slug).invitations(page),
    placeholderData: keepPreviousData,
  });
  const roles = useQuery({
    queryKey: ["club", club.slug, "roles"],
    queryFn: () => api.club(club.slug).roles(),
  });

  const [person, setPerson] = useState(EMPTY_PERSON);
  const [roleId, setRoleId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["club", club.slug, "invitations"] });

  const create = useMutation({
    meta: { inlineError: true },
    mutationFn: () =>
      api.club(club.slug).createInvitation({ ...person, role_id: roleId }),
    onSuccess: () => {
      setError(null);
      setSuccess(t.club.invitations.sent);
      setPerson(EMPTY_PERSON);
      invalidate();
    },
    onError: (e) => {
      setSuccess(null);
      setError(translateApiError(e));
    },
  });
  const resend = useMutation({
    mutationFn: (id: string) => api.club(club.slug).resendInvitation(id),
    onSuccess: invalidate,
  });
  const revoke = useMutation({
    mutationFn: (id: string) => api.club(club.slug).revokeInvitation(id),
    onSuccess: invalidate,
  });

  if (invitations.isPending || roles.isPending) return <Loading />;
  if (invitations.isError || roles.isError) {
    return (
      <ErrorState
        onRetry={() => {
          invitations.refetch();
          roles.refetch();
        }}
      />
    );
  }

  return (
    <>
      {invitations.data.items.length === 0 ? (
        <EmptyState />
      ) : (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-ink-soft">
            {t.common.total(invitations.data.total)}
          </p>
          {invitations.data.items.map((invitation) => (
            <Card
              key={invitation.id}
              className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0">
                <div className="truncate font-semibold">
                  {invitation.first_name} {invitation.last_name}
                </div>
                <div className="truncate text-sm text-ink-soft">
                  {invitation.email}
                </div>
                <div className="text-xs text-ink-soft">
                  {t.club.invitations.expires}{" "}
                  {new Date(invitation.expires_at).toLocaleDateString("fr-FR")}
                </div>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="secondary"
                  disabled={resend.isPending}
                  onClick={() => resend.mutate(invitation.id)}
                >
                  {t.club.invitations.resend}
                </Button>
                <Button
                  variant="danger"
                  disabled={revoke.isPending}
                  onClick={() => revoke.mutate(invitation.id)}
                >
                  {t.club.invitations.revoke}
                </Button>
              </div>
            </Card>
          ))}
          <Pager
            page={page}
            perPage={invitations.data.per_page}
            total={invitations.data.total}
            onPage={setPage}
          />
        </div>
      )}

      <SectionTitle>{t.club.invitations.invite}</SectionTitle>
      <Card>
        <div className="flex flex-col gap-3">
          <PersonFields value={person} onChange={setPerson} />
          <Field label={t.common.role}>
            <Select
              className="w-full"
              value={roleId}
              onChange={(e) => setRoleId(e.target.value)}
            >
              <option value="">{t.common.selectRole}</option>
              {roles.data.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </Select>
          </Field>
          <Button
            disabled={!personComplete(person) || !roleId || create.isPending}
            onClick={() => create.mutate()}
          >
            {t.common.send}
          </Button>
          <FormFeedback error={error} success={success} />
        </div>
      </Card>
    </>
  );
}
