import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router";

import {
  Badge,
  Button,
  Card,
  EmptyState,
  ErrorState,
  Field,
  FormFeedback,
  Loading,
  Pager,
  Select,
  TextInput,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import {
  api,
  setToken,
  type MemberBadges,
  type MemberListItem,
  type RoleDto,
} from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { can, useClub } from "./ClubLayout";

/** Diving badges on the member row (users module). */
function DivingBadges({ badges }: { badges: MemberBadges }) {
  return (
    <>
      {badges.diver && <Badge tone="primary">{badges.diver}</Badge>}
      {badges.teaching && <Badge tone="success">{badges.teaching}</Badge>}
      {badges.gp && <Badge tone="success">{t.diving.badgeGp}</Badge>}
      {badges.has_rifap && <Badge tone="neutral">{t.diving.badgeRifap}</Badge>}
      {badges.medical_expired && (
        <Badge tone="danger">{t.diving.medicalExpired}</Badge>
      )}
      {badges.medical_expired === null && (
        <Badge tone="warning">{t.diving.medicalMissing}</Badge>
      )}
    </>
  );
}

function MemberRow({
  membership,
  slug,
  roles,
  canManageRoles,
  canImpersonate,
}: {
  membership: MemberListItem;
  slug: string;
  roles: RoleDto[];
  canManageRoles: boolean;
  canImpersonate: boolean;
}) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const suspended = membership.status === "suspended";
  const fail = (e: unknown) => setError(translateApiError(e));
  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["club", slug, "members"] });

  const toggleStatus = useMutation({
    meta: { inlineError: true },
    mutationFn: () =>
      api.club(slug).updateMembership(membership.membership_id, {
        status: suspended ? "active" : "suspended",
      }),
    onSuccess: refresh,
    onError: fail,
  });

  // Promote/demote an existing member (e.g. make them Administrateur); the
  // backend enforces the last-admin guard.
  const changeRole = useMutation({
    meta: { inlineError: true },
    mutationFn: (roleId: string) =>
      api
        .club(slug)
        .updateMembership(membership.membership_id, { role_id: roleId }),
    onSuccess: () => {
      setError(null);
      refresh();
    },
    onError: fail,
  });

  const impersonate = async () => {
    try {
      const session = await api.club(slug).impersonate(membership.user.id);
      setToken(session.token);
      queryClient.clear();
      navigate("/");
    } catch (e) {
      fail(e);
    }
  };

  const openProfile = () =>
    navigate(`/clubs/${slug}/members/${membership.user.id}`);

  return (
    <Card>
      {/* The whole identity block opens the member sheet (CACI +
          certifications live there) — a buried "Fiche" button was not
          discoverable (product feedback 2026-08-18). */}
      <button
        type="button"
        onClick={openProfile}
        className="-m-2 flex w-full items-center justify-between gap-2 rounded-(--radius-control) p-2 text-left hover:bg-abysse-50"
      >
        <div className="min-w-0">
          <div className="truncate font-semibold">
            {membership.user.first_name} {membership.user.last_name}
          </div>
          <div className="truncate text-sm text-ink-soft">
            {membership.user.email}
          </div>
          <div className="mt-1 flex flex-wrap gap-2">
            <Badge tone={membership.is_admin ? "primary" : "neutral"}>
              {membership.role_name}
            </Badge>
            {suspended && <Badge tone="warning">{t.common.suspended}</Badge>}
            <DivingBadges badges={membership.badges} />
          </div>
        </div>
        <span
          aria-hidden
          className="shrink-0 text-xl font-semibold text-ink-soft"
        >
          ›
        </span>
      </button>
      <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-hairline pt-3">
        <Button variant="secondary" onClick={openProfile}>
          {t.diving.memberOpenProfile}
        </Button>
        {canManageRoles && (
            <Select
              aria-label={t.club.members.changeRole}
              className="text-sm"
              value={membership.role_id}
              disabled={changeRole.isPending}
              onChange={(e) => changeRole.mutate(e.target.value)}
            >
              {roles.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </Select>
          )}
          {canImpersonate && !suspended && (
            <Button variant="ghost" onClick={impersonate}>
              {t.club.members.impersonate}
            </Button>
          )}
          {canManageRoles && (
            <Button
              variant={suspended ? "secondary" : "danger"}
              disabled={toggleStatus.isPending}
              onClick={() => toggleStatus.mutate()}
            >
              {suspended ? t.club.members.reactivate : t.club.members.suspend}
            </Button>
          )}
      </div>
      {error && <p className="mt-2 text-sm text-danger-600">{error}</p>}
    </Card>
  );
}

/** Directly adds an existing platform user to the club (no email round-trip). */
function AddMemberForm({ slug, roles }: { slug: string; roles: RoleDto[] }) {
  const queryClient = useQueryClient();
  const [email, setEmail] = useState("");
  const [roleId, setRoleId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const add = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.club(slug).addMember(email, roleId),
    onSuccess: () => {
      setError(null);
      setSuccess(t.club.members.added);
      setEmail("");
      queryClient.invalidateQueries({ queryKey: ["club", slug, "members"] });
      queryClient.invalidateQueries({ queryKey: ["club", slug, "invitations"] });
    },
    onError: (e) => {
      setSuccess(null);
      setError(translateApiError(e));
    },
  });
  return (
    <Card className="mt-4">
      <div className="text-sm font-semibold">{t.club.members.addExisting}</div>
      <p className="mt-1 text-xs text-ink-soft">
        {t.club.members.addExistingHint}
      </p>
      <div className="mt-2 flex flex-col gap-3">
        <Field label={t.common.email}>
          <TextInput
            type="email"
            inputMode="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </Field>
        <Field label={t.common.role}>
          <Select
            className="w-full"
            value={roleId}
            onChange={(e) => setRoleId(e.target.value)}
          >
            <option value="">{t.common.selectRole}</option>
            {roles.map((role) => (
              <option key={role.id} value={role.id}>
                {role.name}
              </option>
            ))}
          </Select>
        </Field>
        <Button
          disabled={!email || !roleId || add.isPending}
          onClick={() => add.mutate()}
        >
          {t.club.members.add}
        </Button>
        <FormFeedback error={error} success={success} />
      </div>
    </Card>
  );
}

export default function Members() {
  const club = useClub();
  const canManageRoles = can(club, "club.roles.manage");
  // Member intake (invitation OR direct add) = club.invitations.manage.
  const canIntake = can(club, "club.invitations.manage");
  const [page, setPage] = useState(1);

  const members = useQuery({
    queryKey: ["club", club.slug, "members", page],
    queryFn: () => api.club(club.slug).membersWithBadges(page),
    placeholderData: keepPreviousData,
  });
  const needRoles = canManageRoles || canIntake;
  const roles = useQuery({
    queryKey: ["club", club.slug, "roles"],
    queryFn: () => api.club(club.slug).roles(),
    enabled: needRoles,
  });

  if (members.isPending || (needRoles && roles.isPending)) {
    return <Loading />;
  }
  if (members.isError) return <ErrorState onRetry={() => members.refetch()} />;
  if (needRoles && roles.isError) {
    return <ErrorState onRetry={() => roles.refetch()} />;
  }

  return (
    <div className="flex flex-col gap-2">
      {members.data.items.length === 0 ? (
        <EmptyState />
      ) : (
        <>
          <p className="text-sm text-ink-soft">
            {t.common.total(members.data.total)}
          </p>
          {members.data.items.map((membership) => (
            <MemberRow
              key={membership.membership_id}
              membership={membership}
              slug={club.slug}
              roles={roles.data ?? []}
              canManageRoles={canManageRoles && !!roles.data}
              canImpersonate={can(club, "club.impersonate")}
            />
          ))}
          <Pager
            page={page}
            perPage={members.data.per_page}
            total={members.data.total}
            onPage={setPage}
          />
        </>
      )}
      {canIntake && roles.data && (
        <AddMemberForm slug={club.slug} roles={roles.data} />
      )}
    </div>
  );
}
