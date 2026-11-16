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
  type AddMemberResult,
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

/**
 * Adds a person to the club (ADR-0009: admin-curated accounts). An email
 * that matches an existing account adds that account; otherwise a new one is
 * created — with or without an email. Existing values are never overwritten;
 * differences are listed for review.
 */
function AddMemberForm({ slug, roles }: { slug: string; roles: RoleDto[] }) {
  const queryClient = useQueryClient();
  const defaultRoleId = roles.find((r) => !r.is_admin)?.id ?? "";
  const empty = { first_name: "", last_name: "", email: "", birthdate: "" };
  const [form, setForm] = useState(empty);
  const [roleId, setRoleId] = useState(defaultRoleId);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AddMemberResult | null>(null);
  const set = (key: keyof typeof empty) => (value: string) =>
    setForm((f) => ({ ...f, [key]: value }));
  const add = useMutation({
    meta: { inlineError: true },
    mutationFn: () =>
      api.club(slug).addMember({
        first_name: form.first_name,
        last_name: form.last_name,
        email: form.email.trim() || undefined,
        birthdate: form.birthdate || undefined,
        role_id: roleId || defaultRoleId,
      }),
    onSuccess: (added) => {
      setError(null);
      setResult(added);
      setForm(empty);
      queryClient.invalidateQueries({ queryKey: ["club", slug, "members"] });
    },
    onError: (e) => {
      setResult(null);
      setError(translateApiError(e));
    },
  });
  const m = t.club.members;
  return (
    <Card className="mt-4">
      <div className="text-sm font-semibold">{m.addTitle}</div>
      <p className="mt-1 text-xs text-ink-soft">{m.addHint}</p>
      <div className="mt-2 flex flex-col gap-3">
        <Field label={t.common.lastName}>
          <TextInput
            autoComplete="off"
            value={form.last_name}
            onChange={(e) => set("last_name")(e.target.value)}
          />
        </Field>
        <Field label={t.common.firstName}>
          <TextInput
            autoComplete="off"
            value={form.first_name}
            onChange={(e) => set("first_name")(e.target.value)}
          />
        </Field>
        <Field label={m.emailOptional}>
          <TextInput
            type="email"
            inputMode="email"
            autoComplete="off"
            value={form.email}
            onChange={(e) => set("email")(e.target.value)}
          />
        </Field>
        <Field label={m.birthdateOptional}>
          <TextInput
            type="date"
            value={form.birthdate}
            onChange={(e) => set("birthdate")(e.target.value)}
          />
        </Field>
        <Field label={t.common.role}>
          <Select
            className="w-full"
            value={roleId || defaultRoleId}
            onChange={(e) => setRoleId(e.target.value)}
          >
            {roles.map((role) => (
              <option key={role.id} value={role.id}>
                {role.name}
              </option>
            ))}
          </Select>
        </Field>
        <Button
          disabled={
            !form.first_name.trim() ||
            !form.last_name.trim() ||
            !(roleId || defaultRoleId) ||
            add.isPending
          }
          onClick={() => add.mutate()}
        >
          {m.add}
        </Button>
        <FormFeedback
          error={error}
          success={
            result &&
            (result.outcome === "created" ? m.created : m.addedExisting)
          }
        />
        {result && result.diffs.length > 0 && (
          <div className="rounded-lg bg-warning-50 p-3 text-xs text-ink">
            <div className="font-semibold">{m.diffsTitle}</div>
            <ul className="mt-1 list-disc pl-4">
              {result.diffs.map((d) => (
                <li key={d.field}>
                  {m.diffField[d.field]} : {d.stored} → {m.diffKept}{" "}
                  ({m.diffEntered} {d.incoming})
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Card>
  );
}

export default function Members() {
  const club = useClub();
  const canManageRoles = can(club, "club.roles.manage");
  // Member intake (manual add; later CSV import + access requests) = club.intake.manage.
  const canIntake = can(club, "club.intake.manage");
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
