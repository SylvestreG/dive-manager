import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useState } from "react";
import { Link, Navigate } from "react-router";

import {
  EMPTY_PERSON,
  PersonFields,
  personComplete,
} from "../components/PersonFields";

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
  PageTitle,
  SectionTitle,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, type ClubDto } from "../lib/api";
import { translateApiError } from "../lib/errors";
import { useMe } from "../lib/session";

function useFeedback() {
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const fail = (e: unknown) => setError(translateApiError(e));
  const ok = (message: string) => {
    setError(null);
    setSuccess(message);
  };
  return { error, success, fail, ok };
}

function CreateClubForm() {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const feedback = useFeedback();
  const create = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.platform.createClub(name, slug),
    onSuccess: () => {
      feedback.ok(`${name} ✓`);
      setName("");
      setSlug("");
      queryClient.invalidateQueries({ queryKey: ["platform", "clubs"] });
    },
    onError: feedback.fail,
  });
  return (
    <Card>
      <div className="flex flex-col gap-3">
        <Field label={t.platform.clubName}>
          <TextInput value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label={t.platform.clubSlug}>
          <TextInput
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            placeholder="usi"
          />
        </Field>
        <Button
          disabled={!name || !slug || create.isPending}
          onClick={() => create.mutate()}
        >
          {t.platform.createClub}
        </Button>
        <FormFeedback error={feedback.error} success={feedback.success} />
      </div>
    </Card>
  );
}

function InviteAdminForm({ club }: { club: ClubDto }) {
  const [person, setPerson] = useState(EMPTY_PERSON);
  const feedback = useFeedback();
  const invite = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.platform.inviteClubAdmin(club.id, person),
    onSuccess: () => {
      feedback.ok(t.club.invitations.sent);
      setPerson(EMPTY_PERSON);
    },
    onError: feedback.fail,
  });
  return (
    <div className="mt-3 flex flex-col gap-3 border-t border-hairline pt-3">
      <div className="text-sm font-semibold">{t.platform.inviteAdmin}</div>
      <PersonFields value={person} onChange={setPerson} />
      <Button
        variant="secondary"
        disabled={!personComplete(person) || invite.isPending}
        onClick={() => invite.mutate()}
      >
        {t.common.send}
      </Button>
      <FormFeedback error={feedback.error} success={feedback.success} />
    </div>
  );
}

function ClubRow({ club }: { club: ClubDto }) {
  const queryClient = useQueryClient();
  const [showInvite, setShowInvite] = useState(false);
  const suspended = club.status === "suspended";
  const toggleStatus = useMutation({
    mutationFn: () =>
      api.platform.updateClub(club.id, {
        status: suspended ? "active" : "suspended",
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["platform", "clubs"] }),
  });
  return (
    <Card>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <span className="font-bold">{club.name}</span>{" "}
          <span className="text-sm text-ink-soft">/{club.slug}</span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone={suspended ? "warning" : "success"}>
            {suspended ? t.common.suspended : t.common.active}
          </Badge>
          {!suspended && (
            <Link
              to={`/clubs/${club.slug}`}
              className="min-h-11 content-center px-2 font-semibold text-abysse-700"
            >
              {t.platform.openClub} →
            </Link>
          )}
          <Button
            variant="ghost"
            onClick={() => setShowInvite((v) => !v)}
            disabled={suspended}
          >
            {t.platform.inviteAdmin}
          </Button>
          <Button
            variant={suspended ? "secondary" : "danger"}
            disabled={toggleStatus.isPending}
            onClick={() => toggleStatus.mutate()}
          >
            {suspended ? t.platform.reactivate : t.platform.suspend}
          </Button>
        </div>
      </div>
      {showInvite && !suspended && <InviteAdminForm club={club} />}
    </Card>
  );
}

function GlobalAdmins() {
  const queryClient = useQueryClient();
  const admins = useQuery({
    queryKey: ["platform", "global-admins"],
    queryFn: api.platform.listGlobalAdmins,
  });
  const [person, setPerson] = useState(EMPTY_PERSON);
  const feedback = useFeedback();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["platform", "global-admins"] });
  const add = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.platform.addGlobalAdmin(person),
    onSuccess: () => {
      feedback.ok("✓");
      setPerson(EMPTY_PERSON);
      invalidate();
    },
    onError: feedback.fail,
  });
  const remove = useMutation({
    mutationFn: (userId: string) => api.platform.removeGlobalAdmin(userId),
    onSuccess: invalidate,
  });

  return (
    <>
      <SectionTitle>{t.platform.globalAdmins}</SectionTitle>
      {admins.isPending && <Loading />}
      {admins.isError && <ErrorState onRetry={() => admins.refetch()} />}
      {admins.data && (
        <div className="flex flex-col gap-2">
          {admins.data.map((admin) => (
            <Card key={admin.id} className="flex items-center justify-between">
              <div>
                <span className="font-semibold">
                  {admin.first_name} {admin.last_name}
                </span>{" "}
                <span className="text-sm text-ink-soft">{admin.email}</span>
              </div>
              <Button
                variant="danger"
                disabled={remove.isPending}
                onClick={() => remove.mutate(admin.id)}
              >
                {t.platform.removeGlobalAdmin}
              </Button>
            </Card>
          ))}
          <Card>
            <div className="text-sm font-semibold">
              {t.platform.addGlobalAdmin}
            </div>
            <div className="mt-2 flex flex-col gap-3">
              <PersonFields value={person} onChange={setPerson} />
              <Button
                variant="secondary"
                disabled={!personComplete(person) || add.isPending}
                onClick={() => add.mutate()}
              >
                {t.common.send}
              </Button>
              <FormFeedback error={feedback.error} success={feedback.success} />
            </div>
          </Card>
        </div>
      )}
    </>
  );
}

export default function Platform() {
  const { data: me } = useMe();
  const [page, setPage] = useState(1);
  const isRealGlobalAdmin = !!me && !me.impersonating && me.user.is_global_admin;
  const clubs = useQuery({
    queryKey: ["platform", "clubs", page],
    enabled: isRealGlobalAdmin,
    queryFn: () => api.platform.listClubs(page),
    placeholderData: keepPreviousData,
  });

  // Direct-URL guard: this page is for real (non-impersonating) global
  // admins; the backend refuses everyone else anyway.
  if (me && !isRealGlobalAdmin) {
    return <Navigate to="/" replace />;
  }

  return (
    <>
      <PageTitle>{t.platform.title}</PageTitle>

      <SectionTitle>{t.platform.clubs}</SectionTitle>
      {clubs.isPending && <Loading />}
      {clubs.isError && <ErrorState onRetry={() => clubs.refetch()} />}
      {clubs.data && (
        <div className="flex flex-col gap-2">
          {clubs.data.items.length === 0 && <EmptyState />}
          {clubs.data.items.map((club) => (
            <ClubRow key={club.id} club={club} />
          ))}
          <Pager
            page={page}
            perPage={clubs.data.per_page}
            total={clubs.data.total}
            onPage={setPage}
          />
          <CreateClubForm />
        </div>
      )}

      <GlobalAdmins />
    </>
  );
}
