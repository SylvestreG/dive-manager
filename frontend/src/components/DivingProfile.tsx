// Shared building blocks of the diving profile: the editable profile form,
// the certifications list and the full prerogatives panel. Used by the
// self-service Profile page and the club-side member page.
import { useEffect, useState, type ReactNode } from "react";

import { t } from "../i18n/fr";
import type {
  CertRefDto,
  CertificationDto,
  DerivedDto,
  ProfileFields,
} from "../lib/api";
import { certificationLabel } from "../lib/certifications";
import { translateApiError } from "../lib/errors";
import { Badge, Button, Card, Field, Select, TextInput } from "./ui";

export const STAB_SIZES = ["XS", "S", "M", "L", "XL"];

function Checkbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex min-h-11 items-center gap-3 text-[15px]">
      <input
        type="checkbox"
        className="size-5 accent-abysse-600"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      {label}
    </label>
  );
}

/** Full-replace profile form (mirrors the backend PUT semantics). The CACI
 * date is NOT part of this form: staff record it through the dedicated card
 * on the member page; the value carried in `initial` is resent untouched. */
export function ProfileForm({
  initial,
  preparableOptions,
  isInstructor,
  busy,
  onSubmit,
}: {
  initial: ProfileFields;
  /** Certifications worth preparing (derived server-side: excludes held,
   * covered/redundant, PESH outside the handisub pathway…). */
  preparableOptions: CertRefDto[];
  /** Derived teaching grade present (E1–E4): shows the instructor number
   * field. The backend enforces the actual gate (422 otherwise). */
  isInstructor: boolean;
  busy: boolean;
  onSubmit: (data: ProfileFields) => void;
}) {
  const [data, setData] = useState<ProfileFields>(initial);
  // Re-sync when the server responds (full-replace: the response is truth).
  useEffect(() => setData(initial), [initial]);
  const set = <K extends keyof ProfileFields>(key: K, value: ProfileFields[K]) =>
    setData((d) => ({ ...d, [key]: value }));
  const text = (v: string) => (v.trim() === "" ? null : v);

  return (
    <div className="flex flex-col gap-3">
      <Field label={t.diving.birthdate}>
        <TextInput
          type="date"
          value={data.birthdate ?? ""}
          onChange={(e) => set("birthdate", text(e.target.value))}
        />
      </Field>
      <Field label={t.diving.phone}>
        <TextInput
          type="tel"
          inputMode="tel"
          value={data.phone ?? ""}
          onChange={(e) => set("phone", text(e.target.value))}
        />
      </Field>
      <Field label={t.diving.licenceNumber}>
        <TextInput
          value={data.licence_number ?? ""}
          onChange={(e) => set("licence_number", text(e.target.value))}
        />
      </Field>
      {/* Instructor-only field. Kept visible when a value already exists so
          it can be seen and cleared; hidden otherwise (the value still
          round-trips untouched in the submitted data). */}
      {(isInstructor || initial.instructor_number) && (
        <Field label={t.diving.instructorNumber}>
          <TextInput
            value={data.instructor_number ?? ""}
            onChange={(e) => set("instructor_number", text(e.target.value))}
          />
        </Field>
      )}
      <div className="mt-1 text-sm font-semibold text-ink-soft">
        {t.diving.gear}
      </div>
      <Checkbox
        label={t.diving.gearRegulator}
        checked={data.gear_regulator}
        onChange={(v) => set("gear_regulator", v)}
      />
      <Checkbox
        label={t.diving.gearSecondRegulator}
        checked={data.gear_second_regulator}
        onChange={(v) => set("gear_second_regulator", v)}
      />
      <Checkbox
        label={t.diving.gearStab}
        checked={data.gear_stab}
        onChange={(v) => set("gear_stab", v)}
      />
      {data.gear_stab && (
        <Field label={t.diving.gearStabSize}>
          <Select
            className="w-full"
            value={data.gear_stab_size ?? ""}
            onChange={(e) => set("gear_stab_size", text(e.target.value))}
          >
            <option value="">—</option>
            {STAB_SIZES.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </Select>
        </Field>
      )}
      <Checkbox
        label={t.diving.gearNitrox}
        checked={data.gear_nitrox}
        onChange={(v) => set("gear_nitrox", v)}
      />

      <Checkbox
        label={t.diving.handisub}
        checked={data.handisub}
        onChange={(v) => set("handisub", v)}
      />

      <Field label={t.diving.preparing}>
        <Select
          className="w-full"
          value={data.preparing_certification ?? ""}
          onChange={(e) =>
            set("preparing_certification", text(e.target.value))
          }
        >
          <option value="">{t.diving.preparingNone}</option>
          {/* Keep a stale current value selectable so the form round-trips. */}
          {data.preparing_certification &&
            !preparableOptions.some(
              (c) => c.code === data.preparing_certification,
            ) && (
              <option value={data.preparing_certification}>
                {certificationLabel(data.preparing_certification)}
              </option>
            )}
          {preparableOptions.map((c) => (
            <option key={c.code} value={c.code}>
              {c.label}
            </option>
          ))}
        </Select>
      </Field>

      <Button disabled={busy} onClick={() => onSubmit(data)}>
        {t.common.save}
      </Button>
    </div>
  );
}

export const PROVENANCE_TONE: Record<string, "neutral" | "primary" | "success"> = {
  declared: "neutral",
  recorded: "primary",
  imported: "primary",
  validated: "success",
};

export function CertificationList({
  certifications,
  canDelete,
  onDelete,
  deleting,
}: {
  certifications: CertificationDto[];
  canDelete: (cert: CertificationDto) => boolean;
  onDelete: (cert: CertificationDto) => void;
  deleting: boolean;
}) {
  if (certifications.length === 0) {
    return <p className="text-sm text-ink-soft">{t.common.emptyList}</p>;
  }
  return (
    <ul className="flex flex-col divide-y divide-hairline">
      {certifications.map((cert) => (
        <li
          key={cert.id}
          className="flex flex-col gap-2 py-3 first:pt-0 last:pb-0 sm:flex-row sm:items-center sm:justify-between"
        >
          <div className="min-w-0">
            <div className="font-semibold">{cert.label}</div>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-ink-soft">
              <Badge tone={PROVENANCE_TONE[cert.provenance] ?? "neutral"}>
                {t.diving.provenance[cert.provenance] ?? cert.provenance}
              </Badge>
              {cert.obtained_on && <span>{cert.obtained_on}</span>}
            </div>
          </div>
          {canDelete(cert) && (
            <Button
              variant="danger"
              disabled={deleting}
              onClick={() => onDelete(cert)}
            >
              {t.diving.remove}
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}

const APTITUDE_TONE: Record<string, "primary" | "success" | "warning"> = {
  guided: "primary",
  autonomous: "success",
  pesh: "warning",
};

/** ALL derived prerogatives (product requirement: the profile page shows
 * everything the person may do). Pure display over DerivedDto. */
export function PrerogativesPanel({ derived }: { derived: DerivedDto }) {
  if (derived.incomplete) return null;
  const dpRows = [
    { ok: derived.dp_pool, label: t.diving.dpPool },
    { ok: derived.dp_exploration, label: t.diving.dpExploration },
    { ok: derived.dp_teaching, label: t.diving.dpTeaching },
    // "All zones" implies the limited zone: show the widest one only.
    {
      ok: derived.dp_trimix_shallow && !derived.dp_trimix,
      label: t.diving.dpTrimixShallow,
    },
    { ok: derived.dp_trimix, label: t.diving.dpTrimix },
  ].filter((row) => row.ok);

  return (
    <div className="flex flex-col gap-4">
      <div>
        <div className="mb-2 text-sm font-semibold text-ink-soft">
          {t.diving.aptitudes}
          {derived.age !== null && (
            <span className="ml-2 font-normal">({t.diving.age(derived.age)})</span>
          )}
        </div>
        {derived.aptitudes.length === 0 ? (
          <p className="text-sm text-ink-soft">{t.diving.noAptitude}</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {derived.aptitudes.map((a) => (
              <Badge key={a.code} tone={APTITUDE_TONE[a.kind]}>
                {a.code} · {t.diving.aptitudeKind[a.kind]} ·{" "}
                {t.diving.depth(a.max_depth_m)}
              </Badge>
            ))}
          </div>
        )}
      </div>

      {derived.restrictions.length > 0 && (
        <div>
          <div className="mb-1 text-sm font-semibold text-ink-soft">
            {t.diving.restrictions}
          </div>
          <ul className="list-inside list-disc text-sm text-warning-600">
            {derived.restrictions.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        </div>
      )}

      <div>
        <div className="mb-1 text-sm font-semibold text-ink-soft">
          {t.diving.supervision}
        </div>
        {derived.gp || derived.teaching ? (
          <div className="flex flex-wrap gap-2">
            {derived.gp && <Badge tone="success">{t.diving.gpYes}</Badge>}
            {derived.teaching && (
              <Badge tone="success">
                {t.diving.teachingGrade(derived.teaching)}
              </Badge>
            )}
          </div>
        ) : (
          <p className="text-sm text-ink-soft">{t.diving.noSupervision}</p>
        )}
      </div>

      <div>
        <div className="mb-1 text-sm font-semibold text-ink-soft">
          {t.diving.dp}
        </div>
        {dpRows.length === 0 ? (
          <p className="text-sm text-ink-soft">{t.diving.noDp}</p>
        ) : (
          <>
            <div className="flex flex-wrap gap-2">
              {dpRows.map((row) => (
                <Badge key={row.label} tone="primary">
                  {row.label}
                </Badge>
              ))}
            </div>
            {!derived.dp_mixtures_ok && (
              <p className="mt-1 text-xs text-ink-soft">{t.diving.dpMixtures}</p>
            )}
          </>
        )}
      </div>

      <div>
        <div className="mb-1 text-sm font-semibold text-ink-soft">
          {t.diving.gas}
        </div>
        <Badge tone="primary">{t.diving.maxO2(derived.max_o2_percent)}</Badge>
      </div>

      <div>
        <div className="mb-1 text-sm font-semibold text-ink-soft">
          {t.diving.eligibleNext}
        </div>
        {derived.eligible_next.length === 0 ? (
          <p className="text-sm text-ink-soft">{t.diving.noEligible}</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {derived.eligible_next.map((c) => (
              <Badge key={c.code}>{c.label}</Badge>
            ))}
          </div>
        )}
      </div>

      {derived.preparing_issues.length > 0 && (
        <div>
          <div className="mb-1 text-sm font-semibold text-ink-soft">
            {t.diving.preparingIssues}
          </div>
          <ul className="list-inside list-disc text-sm text-ink-soft">
            {derived.preparing_issues.map((issue) => (
              <li key={issue}>{issue}</li>
            ))}
          </ul>
        </div>
      )}

      <p className="text-xs text-ink-soft">
        {t.diving.regulationNote(derived.regulation_snapshot)}
      </p>
    </div>
  );
}

/** Red banner for a missing birthdate (product decision 2026-08-17). */
export function IncompleteBanner({
  message,
  cta,
}: {
  message: string;
  cta?: ReactNode;
}) {
  return (
    <Card className="border-danger-600/40 bg-danger-50">
      <p className="text-sm font-semibold text-danger-600">{message}</p>
      {cta}
    </Card>
  );
}

const MAX_STAMP_BYTES = 512 * 1024;

/** File → base64 via the browser's native encoder (data URL, prefix
 * stripped). */
function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      resolve(dataUrl.slice(dataUrl.indexOf(",") + 1));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

/** Instructor stamp: preview + PNG/JPEG upload (base64) + delete. The caller
 * provides the API calls so this works for /me and for the club member page. */
export function StampSection({
  hasStamp,
  fetchStamp,
  uploadStamp,
  deleteStamp,
  onChanged,
}: {
  hasStamp: boolean;
  fetchStamp: () => Promise<Blob | null>;
  uploadStamp: (imageBase64: string) => Promise<unknown>;
  deleteStamp: () => Promise<unknown>;
  onChanged: () => void;
}) {
  const [preview, setPreview] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Bumped on every successful upload so the preview refetches even when
  // hasStamp stays true (replace case).
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let url: string | null = null;
    let cancelled = false;
    if (hasStamp) {
      fetchStamp()
        .then((blob) => {
          if (cancelled || !blob) return;
          url = URL.createObjectURL(blob);
          setPreview(url);
        })
        .catch(() => setPreview(null));
    } else {
      setPreview(null);
    }
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasStamp, version]);

  const onFile = async (file: File | undefined) => {
    if (!file) return;
    setError(null);
    if (file.size > MAX_STAMP_BYTES) {
      setError(t.diving.stampTooBig);
      return;
    }
    setBusy(true);
    try {
      await uploadStamp(await fileToBase64(file));
      setVersion((v) => v + 1);
      onChanged();
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setError(null);
    try {
      await deleteStamp();
      setPreview(null);
      onChanged();
    } catch (e) {
      setError(translateApiError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-ink-soft">{t.diving.stampHint}</p>
      {preview && (
        <img
          src={preview}
          alt={t.diving.stamp}
          className="max-h-32 w-fit rounded-(--radius-control) border border-hairline bg-white p-2"
        />
      )}
      <div className="flex flex-wrap gap-2">
        <label className="inline-flex min-h-11 cursor-pointer items-center justify-center gap-2 rounded-(--radius-control) border border-hairline bg-white px-4 text-[15px] font-semibold text-abysse-700 hover:bg-abysse-50">
          {hasStamp ? t.diving.stampReplace : t.diving.stampUpload}
          <input
            type="file"
            accept="image/png,image/jpeg"
            className="hidden"
            disabled={busy}
            onChange={(e) => onFile(e.target.files?.[0])}
          />
        </label>
        {hasStamp && (
          <Button variant="danger" disabled={busy} onClick={remove}>
            {t.diving.stampDelete}
          </Button>
        )}
      </div>
      {error && <p className="text-sm font-medium text-danger-600">{error}</p>}
    </div>
  );
}

/** CACI status line (read-only display; edited by club staff). */
export function MedicalStatus({
  validUntil,
  expired,
  showManagedHint,
}: {
  validUntil: string | null;
  expired: boolean | null;
  showManagedHint: boolean;
}) {
  return (
    <div>
      <div className="mb-1 text-sm font-semibold text-ink-soft">
        {t.diving.medical}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {validUntil ? (
          <>
            <span className="text-sm">
              {t.diving.medicalValidUntil} {validUntil}
            </span>
            {expired && <Badge tone="danger">{t.diving.medicalExpired}</Badge>}
          </>
        ) : (
          <Badge tone="warning">{t.diving.medicalMissing}</Badge>
        )}
      </div>
      {showManagedHint && (
        <p className="mt-1 text-xs text-ink-soft">
          {t.diving.medicalManagedByClub}
        </p>
      )}
    </div>
  );
}
