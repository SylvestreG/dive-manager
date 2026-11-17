import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router";

import {
  CertificationList,
  IncompleteBanner,
  MedicalStatus,
  PrerogativesPanel,
  ProfileForm,
  StampSection,
} from "../components/DivingProfile";
import {
  Button,
  Card,
  ErrorState,
  Field,
  FormFeedback,
  Loading,
  PageTitle,
  SectionTitle,
  TextInput,
} from "../components/ui";
import { t } from "../i18n/fr";
import {
  api,
  clearToken,
  type FfessmImportResult,
  type ProfileResponse,
} from "../lib/api";
import { translateApiError } from "../lib/errors";
import { decodeQrFromFile } from "../lib/qr";
import { useMe } from "../lib/session";
import { Badge } from "../components/ui";

/** Identity (first/last name) — editable: bootstrap accounts start with a
 * name derived from the email, and the licence import matches on it. */
function IdentityCard({
  firstName,
  lastName,
  email,
}: {
  firstName: string;
  lastName: string;
  email: string | null;
}) {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [first, setFirst] = useState(firstName);
  const [last, setLast] = useState(lastName);
  const [error, setError] = useState<string | null>(null);
  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.updateMyIdentity(first, last),
    onSuccess: () => {
      setError(null);
      setEditing(false);
      queryClient.invalidateQueries({ queryKey: ["me"] });
    },
    onError: (e) => setError(translateApiError(e)),
  });

  return (
    <Card>
      {editing ? (
        <div className="flex flex-col gap-3">
          <Field label={t.common.firstName}>
            <TextInput value={first} onChange={(e) => setFirst(e.target.value)} />
          </Field>
          <Field label={t.common.lastName}>
            <TextInput value={last} onChange={(e) => setLast(e.target.value)} />
          </Field>
          <div className="flex gap-2">
            <Button
              disabled={!first.trim() || !last.trim() || save.isPending}
              onClick={() => save.mutate()}
            >
              {t.common.save}
            </Button>
            <Button variant="secondary" onClick={() => setEditing(false)}>
              {t.common.cancel}
            </Button>
          </div>
          <FormFeedback error={error} />
        </div>
      ) : (
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <div className="truncate font-bold">
              {firstName} {lastName}
            </div>
            {email && (
              <div className="truncate text-sm text-ink-soft">{email}</div>
            )}
          </div>
          <Button
            variant="ghost"
            onClick={() => {
              setFirst(firstName);
              setLast(lastName);
              setEditing(true);
            }}
          >
            {t.profile.editIdentity}
          </Button>
        </div>
      )}
    </Card>
  );
}

/** Scan/paste MY official FFESSM licence to auto-fill profile + brevets.
 * Self-service (product decision 2026-08-18): the federation is the source
 * of truth, provenance `imported`. */
function FfessmImportCard({
  onDone,
}: {
  onDone: (data: ProfileResponse) => void;
}) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<FfessmImportResult | null>(null);
  const run = useMutation({
    meta: { inlineError: true },
    mutationFn: (value: string) =>
      api.importMyFfessmLicence({ url: value.trim() }),
    onSuccess: (data) => {
      setError(null);
      setResult(data);
      onDone(data.profile);
    },
    onError: (e) => {
      setResult(null);
      setError(translateApiError(e));
    },
  });

  // Photo of the licence QR → decode client-side → import immediately.
  const onScanPhoto = async (file: File | undefined) => {
    if (!file) return;
    setError(null);
    setScanning(true);
    try {
      const decoded = await decodeQrFromFile(file);
      if (!decoded) {
        setError(t.diving.ffessm.scanFailed);
        return;
      }
      setUrl(decoded);
      run.mutate(decoded);
    } finally {
      setScanning(false);
    }
  };

  const busy = scanning || run.isPending;

  return (
    <Card>
      <div className="text-sm font-semibold">{t.diving.ffessm.title}</div>
      <p className="mt-1 text-xs text-ink-soft">{t.diving.ffessm.hint}</p>
      {/* The primary path on a phone: shoot the QR with the native camera. */}
      <label
        className={`mt-3 inline-flex min-h-11 items-center justify-center gap-2 rounded-(--radius-control) bg-abysse-600 px-4 text-[15px] font-semibold text-white hover:bg-abysse-700 ${
          busy ? "cursor-not-allowed opacity-60" : "cursor-pointer"
        }`}
      >
        <span aria-hidden>📷</span>
        {scanning ? t.diving.ffessm.scanning : t.diving.ffessm.scan}
        <input
          type="file"
          accept="image/*"
          capture="environment"
          className="hidden"
          disabled={busy}
          onChange={(e) => {
            void onScanPhoto(e.target.files?.[0]);
            e.target.value = "";
          }}
        />
      </label>
      <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-end">
        <div className="flex-1">
          <Field label={t.diving.ffessm.urlLabel}>
            <TextInput
              type="url"
              inputMode="url"
              autoComplete="off"
              placeholder={t.diving.ffessm.urlPlaceholder}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
            />
          </Field>
        </div>
        <Button disabled={!url.trim() || busy} onClick={() => run.mutate(url)}>
          {run.isPending ? t.diving.ffessm.importing : t.diving.ffessm.import}
        </Button>
      </div>
      {result && (
        <div className="mt-3 flex flex-col gap-2 text-sm">
          <Badge tone="success">
            {result.holder_name
              ? t.diving.ffessm.resultFor(result.holder_name)
              : t.diving.ffessm.imported(result.imported.length)}
          </Badge>
          <div className="text-ink-soft">
            {t.diving.ffessm.imported(result.imported.length)}
          </div>
          {result.skipped_stronger.length > 0 && (
            <div className="text-ink-soft">
              {t.diving.ffessm.skipped(result.skipped_stronger.join(", "))}
            </div>
          )}
          {result.unmapped.length > 0 && (
            <Badge tone="warning">
              {t.diving.ffessm.unmapped(result.unmapped.join(", "))}
            </Badge>
          )}
        </div>
      )}
      <FormFeedback error={error} />
    </Card>
  );
}

export default function Profile() {
  const { data: me } = useMe();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [profileError, setProfileError] = useState<string | null>(null);
  const [profileSaved, setProfileSaved] = useState(false);

  const profile = useQuery({
    queryKey: ["me", "profile"],
    queryFn: () => api.myProfile(),
  });

  const refreshProfile = (data?: ProfileResponse) => {
    if (data) queryClient.setQueryData(["me", "profile"], data);
    else queryClient.invalidateQueries({ queryKey: ["me", "profile"] });
  };

  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: api.updateMyProfile,
    onSuccess: (data) => {
      setProfileError(null);
      setProfileSaved(true);
      refreshProfile(data);
    },
    onError: (e) => {
      setProfileSaved(false);
      setProfileError(translateApiError(e));
    },
  });

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

  if (!me) return null;
  if (profile.isPending) return <Loading />;
  if (profile.isError) {
    return <ErrorState onRetry={() => profile.refetch()} />;
  }
  const data = profile.data;

  return (
    <>
      <PageTitle>{t.profile.title}</PageTitle>
      {data.derived.incomplete && (
        <div className="mb-4">
          <IncompleteBanner message={t.diving.incompleteBanner} />
        </div>
      )}
      <IdentityCard
        firstName={me.user.first_name}
        lastName={me.user.last_name}
        email={me.user.email}
      />

      {/* Each member imports their OWN licence (product decision
          2026-08-18) — first thing on the page, it fills everything. */}
      <div className="mt-4">
        <FfessmImportCard onDone={(p) => refreshProfile(p)} />
      </div>

      <SectionTitle>{t.diving.sectionProfile}</SectionTitle>
      <Card>
        <div className="mb-4">
          <MedicalStatus
            validUntil={data.profile.medical_certificate_valid_until}
            expired={data.derived.medical_expired}
            showManagedHint
          />
        </div>
        <ProfileForm
          initial={data.profile}
          preparableOptions={data.derived.preparable}
          isInstructor={data.derived.teaching !== null}
          busy={save.isPending}
          onSubmit={(fields) => save.mutate(fields)}
        />
        <FormFeedback
          error={profileError}
          success={profileSaved ? t.diving.saved : null}
        />
      </Card>

      <SectionTitle>{t.diving.sectionCertifications}</SectionTitle>
      <Card>
        {/* Read-only: certifications are entered by club staff (product
            decision 2026-08-18). */}
        <CertificationList
          certifications={data.certifications}
          canDelete={() => false}
          onDelete={() => {}}
          deleting={false}
        />
        <p className="mt-3 text-xs text-ink-soft">
          {t.diving.certificationsManagedByClub}
        </p>
      </Card>

      <SectionTitle>{t.diving.sectionPrerogatives}</SectionTitle>
      <Card>
        {data.derived.incomplete ? (
          <p className="text-sm text-ink-soft">{t.diving.incompleteMember}</p>
        ) : (
          <PrerogativesPanel derived={data.derived} />
        )}
      </Card>

      {/* Stamp: shown for instructors (E2+) or when a stamp already exists —
          the backend enforces the actual gate. */}
      {(data.has_stamp || data.derived.teaching) && (
        <>
          <SectionTitle>{t.diving.stamp}</SectionTitle>
          <Card>
            <StampSection
              hasStamp={data.has_stamp}
              fetchStamp={api.myStamp}
              uploadStamp={api.putMyStamp}
              deleteStamp={api.deleteMyStamp}
              onChanged={() => refreshProfile()}
            />
          </Card>
        </>
      )}


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
