// Club-side member sheet: diving profile, certifications with provenance,
// CACI (recorded here — product decision 2026-08-17), stagiaire-MF1 status,
// stamp, and the full derived prerogatives.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useParams } from "react-router";

import {
  CertificationTree,
  type CertificationChanges,
} from "../../components/CertificationTree";
import {
  CertificationList,
  IncompleteBanner,
  MedicalStatus,
  PrerogativesPanel,
  ProfileForm,
  StampSection,
} from "../../components/DivingProfile";
import {
  Badge,
  Button,
  Card,
  ErrorState,
  Field,
  FormFeedback,
  Loading,
  PageTitle,
  SectionTitle,
  TextInput,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type ProfileResponse } from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { can, useClub } from "./ClubLayout";

/** Dedicated CACI card — the main reason staff open this page. Reuses the
 * full-replace profile PUT with only the medical date changed. */
function MedicalCard({
  slug,
  userId,
  profile,
  expired,
  onDone,
}: {
  slug: string;
  userId: string;
  profile: ProfileResponse["profile"];
  expired: boolean | null;
  onDone: (data: ProfileResponse) => void;
}) {
  const [date, setDate] = useState(profile.medical_certificate_valid_until ?? "");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  useEffect(
    () => setDate(profile.medical_certificate_valid_until ?? ""),
    [profile.medical_certificate_valid_until],
  );
  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: (value: string | null) =>
      api.club(slug).updateMemberProfile(userId, {
        ...profile,
        medical_certificate_valid_until: value,
      }),
    onSuccess: (data) => {
      setError(null);
      setSaved(true);
      onDone(data);
    },
    onError: (e) => {
      setSaved(false);
      setError(translateApiError(e));
    },
  });

  return (
    <Card>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-semibold">{t.diving.medical}</span>
        {profile.medical_certificate_valid_until === null && (
          <Badge tone="warning">{t.diving.medicalMissing}</Badge>
        )}
        {expired && <Badge tone="danger">{t.diving.medicalExpired}</Badge>}
      </div>
      <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-end">
        <div className="flex-1">
          <Field label={t.diving.medicalValidUntil}>
            <TextInput
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </Field>
        </div>
        <Button
          disabled={!date || save.isPending}
          onClick={() => save.mutate(date)}
        >
          {t.common.save}
        </Button>
      </div>
      <FormFeedback error={error} success={saved ? t.diving.medicalSaved : null} />
    </Card>
  );
}

function TrainingStatusSection({
  slug,
  userId,
  profile,
  onDone,
}: {
  slug: string;
  userId: string;
  profile: ProfileResponse;
  onDone: (data: ProfileResponse) => void;
}) {
  const current = profile.training_statuses.find(
    (s) => s.status === "stagiaire_mf1",
  );
  const [validUntil, setValidUntil] = useState(current?.valid_until ?? "");
  const [error, setError] = useState<string | null>(null);
  const set = useMutation({
    meta: { inlineError: true },
    mutationFn: (value: string | null) =>
      api.club(slug).setTrainingStatus(userId, value),
    onSuccess: (data) => {
      setError(null);
      onDone(data);
    },
    onError: (e) => setError(translateApiError(e)),
  });

  return (
    <Card>
      <p className="text-xs text-ink-soft">{t.diving.trainingStatusHint}</p>
      {current && (
        <div className="mt-2">
          <Badge tone="success">
            {t.diving.trainingStatusUntil} {current.valid_until}
          </Badge>
        </div>
      )}
      <div className="mt-3 flex flex-col gap-3">
        <Field label={t.diving.trainingStatusUntil}>
          <TextInput
            type="date"
            value={validUntil}
            onChange={(e) => setValidUntil(e.target.value)}
          />
        </Field>
        <div className="flex flex-wrap gap-2">
          <Button
            disabled={!validUntil || set.isPending}
            onClick={() => set.mutate(validUntil)}
          >
            {t.diving.trainingStatusSet}
          </Button>
          {current && (
            <Button
              variant="danger"
              disabled={set.isPending}
              onClick={() => set.mutate(null)}
            >
              {t.diving.trainingStatusRemove}
            </Button>
          )}
        </div>
        <FormFeedback error={error} />
      </div>
    </Card>
  );
}

export default function MemberProfile() {
  const club = useClub();
  const { userId } = useParams<{ userId: string }>();
  const queryClient = useQueryClient();
  const canManage = can(club, "club.members.manage");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [certError, setCertError] = useState<string | null>(null);
  const [certSaved, setCertSaved] = useState(false);

  const profile = useQuery({
    queryKey: ["club", club.slug, "member-profile", userId],
    queryFn: () => api.club(club.slug).memberProfile(userId!),
    enabled: !!userId,
  });

  // Static FFESSM catalog (codes + coverage) for the certification tree.
  const catalog = useQuery({
    queryKey: ["catalog", "certifications"],
    queryFn: api.certificationCatalog,
    staleTime: Infinity,
    enabled: canManage,
  });

  const refresh = (data?: ProfileResponse) => {
    if (data) {
      queryClient.setQueryData(
        ["club", club.slug, "member-profile", userId],
        data,
      );
    } else {
      queryClient.invalidateQueries({
        queryKey: ["club", club.slug, "member-profile", userId],
      });
    }
    // Badges on the members list depend on this data.
    queryClient.invalidateQueries({ queryKey: ["club", club.slug, "members"] });
  };

  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: (fields: ProfileResponse["profile"]) =>
      api.club(club.slug).updateMemberProfile(userId!, fields),
    onSuccess: (data) => {
      setSaveError(null);
      setSaved(true);
      refresh(data);
    },
    onError: (e) => {
      setSaved(false);
      setSaveError(translateApiError(e));
    },
  });

  // Applies the staged tree changes sequentially (each call full-replays the
  // profile payload server-side; order: additions then removals).
  const saveCerts = useMutation({
    meta: { inlineError: true },
    mutationFn: async (changes: CertificationChanges) => {
      for (const addition of changes.additions) {
        await api.club(club.slug).recordCertification(userId!, {
          certification: addition.certification,
          obtained_on: addition.obtained_on,
          validated: changes.validated,
        });
      }
      for (const rowId of changes.removals) {
        await api.club(club.slug).deleteMemberCertification(userId!, rowId);
      }
    },
    onSuccess: () => {
      setCertError(null);
      setCertSaved(true);
      refresh();
    },
    onError: (e) => {
      setCertSaved(false);
      setCertError(translateApiError(e));
      // Some writes may have landed before the failure: reload the truth.
      refresh();
    },
  });

  if (profile.isPending) return <Loading />;
  if (profile.isError) {
    return (
      <ErrorState
        message={translateApiError(profile.error)}
        onRetry={() => profile.refetch()}
      />
    );
  }
  const data = profile.data;

  return (
    <div className="flex flex-col gap-2">
      <PageTitle>{t.diving.memberProfileTitle}</PageTitle>
      {data.derived.incomplete && (
        <IncompleteBanner message={t.diving.incompleteMember} />
      )}
      <Card>
        <div className="font-bold">
          {data.user.first_name} {data.user.last_name}
        </div>
        <div className="text-sm text-ink-soft">{data.user.email}</div>
      </Card>

      {/* The FFESSM licence import is self-service on the member's own
          profile page (product decision 2026-08-18) — nothing here. */}

      {/* CACI first: recording it is the main staff action on this page
          (product decision 2026-08-17). */}
      {canManage && (
        <MedicalCard
          slug={club.slug}
          userId={userId!}
          profile={data.profile}
          expired={data.derived.medical_expired}
          onDone={refresh}
        />
      )}

      <SectionTitle>{t.diving.sectionProfile}</SectionTitle>
      <Card>
        {canManage ? (
          <>
            <ProfileForm
              initial={data.profile}
              preparableOptions={data.derived.preparable}
              isInstructor={data.derived.teaching !== null}
              busy={save.isPending}
              onSubmit={(fields) => save.mutate(fields)}
            />
            <FormFeedback
              error={saveError}
              success={saved ? t.diving.saved : null}
            />
          </>
        ) : (
          <div className="flex flex-col gap-3 text-sm">
            <MedicalStatus
              validUntil={data.profile.medical_certificate_valid_until}
              expired={data.derived.medical_expired}
              showManagedHint={false}
            />
            <div>
              <span className="font-semibold text-ink-soft">
                {t.diving.birthdate} :
              </span>{" "}
              {data.profile.birthdate ?? "—"}
            </div>
            <div>
              <span className="font-semibold text-ink-soft">
                {t.diving.phone} :
              </span>{" "}
              {data.profile.phone ?? "—"}
            </div>
            <div>
              <span className="font-semibold text-ink-soft">
                {t.diving.licenceNumber} :
              </span>{" "}
              {data.profile.licence_number ?? "—"}
            </div>
          </div>
        )}
      </Card>

      <SectionTitle>{t.diving.sectionCertifications}</SectionTitle>
      <Card>
        {canManage ? (
          catalog.isPending ? (
            <Loading />
          ) : catalog.isError ? (
            <ErrorState
              message={translateApiError(catalog.error)}
              onRetry={() => catalog.refetch()}
            />
          ) : (
            <>
              <CertificationTree
                certifications={data.certifications}
                catalog={catalog.data}
                busy={saveCerts.isPending}
                onSave={(changes) => saveCerts.mutate(changes)}
              />
              <FormFeedback
                error={certError}
                success={certSaved ? t.diving.certificationsSaved : null}
              />
            </>
          )
        ) : (
          <CertificationList
            certifications={data.certifications}
            canDelete={() => false}
            onDelete={() => {}}
            deleting={false}
          />
        )}
      </Card>

      {canManage && (
        <>
          <SectionTitle>{t.diving.trainingStatus}</SectionTitle>
          <TrainingStatusSection
            slug={club.slug}
            userId={userId!}
            profile={data}
            onDone={refresh}
          />
        </>
      )}

      <SectionTitle>{t.diving.sectionPrerogativesMember}</SectionTitle>
      <Card>
        {data.derived.incomplete ? (
          <p className="text-sm text-ink-soft">{t.diving.incompleteMember}</p>
        ) : (
          <PrerogativesPanel derived={data.derived} />
        )}
      </Card>

      {(data.has_stamp || (canManage && data.derived.teaching)) && (
        <>
          <SectionTitle>{t.diving.stamp}</SectionTitle>
          <Card>
            {canManage ? (
              <StampSection
                hasStamp={data.has_stamp}
                fetchStamp={() => api.club(club.slug).memberStamp(userId!)}
                uploadStamp={(image) =>
                  api.club(club.slug).putMemberStamp(userId!, image)
                }
                deleteStamp={() => api.club(club.slug).deleteMemberStamp(userId!)}
                onChanged={() => refresh()}
              />
            ) : (
              <StampView slug={club.slug} userId={userId!} />
            )}
          </Card>
        </>
      )}
    </div>
  );
}

/** Read-only stamp preview for staff with view-only permission. Plain
 * effect (not useQuery): an objectURL must be revoked on unmount, which a
 * query cache would keep alive. */
function StampView({ slug, userId }: { slug: string; userId: string }) {
  const [preview, setPreview] = useState<string | null>(null);
  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    api
      .club(slug)
      .memberStamp(userId)
      .then((blob) => {
        if (cancelled || !blob) return;
        url = URL.createObjectURL(blob);
        setPreview(url);
      })
      .catch(() => setPreview(null));
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [slug, userId]);
  if (!preview) return null;
  return (
    <img
      src={preview}
      alt={t.diving.stamp}
      className="max-h-32 w-fit rounded-(--radius-control) border border-hairline bg-white p-2"
    />
  );
}
