// Certification checkbox tree (product decision 2026-08-18): staff check the
// certifications a member explicitly holds; everything a checked one covers
// shows checked but greyed ("acquis via la sélection") — derived from the
// catalog's `covers`, never stored. Changes are staged locally (additions
// with an optional date, removals of existing rows) and applied in one save.
import { useEffect, useMemo, useState } from "react";

import { t } from "../i18n/fr";
import type { CatalogEntry, CertificationDto } from "../lib/api";
import { CERTIFICATION_GROUPS, CERTIFICATIONS } from "../lib/certifications";
import { PROVENANCE_TONE } from "./DivingProfile";
import { Badge, Button, Field, TextInput } from "./ui";

export interface CertificationChanges {
  additions: { certification: string; obtained_on: string | null }[];
  /** Ids of existing certification ROWS to delete. */
  removals: string[];
  /** Provenance flag applied to ALL staged additions. */
  validated: boolean;
}

/** Codes covered (transitively) by the given checked codes. */
function coverage(
  checked: Iterable<string>,
  coversByCode: Map<string, string[]>,
): Set<string> {
  const covered = new Set<string>();
  const stack = [...checked];
  while (stack.length > 0) {
    const code = stack.pop()!;
    for (const dep of coversByCode.get(code) ?? []) {
      if (!covered.has(dep)) {
        covered.add(dep);
        stack.push(dep);
      }
    }
  }
  return covered;
}

/** Drops staged additions that became covered by the rest of the selection
 * (checking "N2" absorbs a staged "PE40"), to a fixpoint. */
function pruneCoveredAdditions(
  additions: Record<string, string>,
  baseCodes: string[],
  coversByCode: Map<string, string[]>,
): Record<string, string> {
  const kept = { ...additions };
  let changed = true;
  while (changed) {
    changed = false;
    for (const code of Object.keys(kept)) {
      const others = [
        ...baseCodes,
        ...Object.keys(kept).filter((c) => c !== code),
      ];
      if (coverage(others, coversByCode).has(code)) {
        delete kept[code];
        changed = true;
      }
    }
  }
  return kept;
}

export function CertificationTree({
  certifications,
  catalog,
  busy,
  onSave,
}: {
  certifications: CertificationDto[];
  catalog: CatalogEntry[];
  busy: boolean;
  onSave: (changes: CertificationChanges) => void;
}) {
  // Staged addition codes → obtained_on ("" = not provided).
  const [additions, setAdditions] = useState<Record<string, string>>({});
  // Staged removals: ids of existing rows.
  const [removals, setRemovals] = useState<string[]>([]);
  const [validated, setValidated] = useState(false);

  // Staged changes are relative to the held rows: reset when they change
  // (after a save the refreshed profile is the new truth).
  const heldSignature = certifications.map((c) => c.id).join(",");
  useEffect(() => {
    setAdditions({});
    setRemovals([]);
    setValidated(false);
  }, [heldSignature]);

  const coversByCode = useMemo(
    () => new Map(catalog.map((entry) => [entry.code, entry.covers])),
    [catalog],
  );
  const catalogByCode = useMemo(
    () => new Map(catalog.map((entry) => [entry.code, entry])),
    [catalog],
  );
  const heldByCode = useMemo(() => {
    const map = new Map<string, CertificationDto>();
    for (const cert of certifications) {
      if (!map.has(cert.code)) map.set(cert.code, cert);
    }
    return map;
  }, [certifications]);

  // Effective explicit selection: held minus staged removals, plus staged
  // additions. Coverage is derived from it.
  const effectiveHeldCodes = certifications
    .filter((c) => !removals.includes(c.id))
    .map((c) => c.code);
  const covered = coverage(
    [...effectiveHeldCodes, ...Object.keys(additions)],
    coversByCode,
  );

  const toggle = (code: string) => {
    const row = heldByCode.get(code);
    if (row) {
      const nextRemovals = removals.includes(row.id)
        ? removals.filter((id) => id !== row.id)
        : [...removals, row.id];
      setRemovals(nextRemovals);
      const baseCodes = certifications
        .filter((c) => !nextRemovals.includes(c.id))
        .map((c) => c.code);
      setAdditions((current) =>
        pruneCoveredAdditions(current, baseCodes, coversByCode),
      );
      return;
    }
    setAdditions((current) => {
      const next = { ...current };
      if (code in next) {
        delete next[code];
      } else {
        next[code] = "";
      }
      return pruneCoveredAdditions(next, effectiveHeldCodes, coversByCode);
    });
  };

  // Static-file order inside groups; only codes the server knows about.
  const entries = CERTIFICATIONS.filter((ref) => catalogByCode.has(ref.code));
  const hasChanges = Object.keys(additions).length > 0 || removals.length > 0;

  return (
    <div className="flex flex-col gap-3">
      <label className="flex min-h-11 items-center gap-3 text-[15px]">
        <input
          type="checkbox"
          className="size-5 accent-abysse-600"
          checked={validated}
          disabled={busy}
          onChange={(e) => setValidated(e.target.checked)}
        />
        {t.diving.recordValidated}
      </label>

      {CERTIFICATION_GROUPS.map((group) => {
        const groupEntries = entries.filter((entry) => entry.group === group);
        if (groupEntries.length === 0) return null;
        return (
          <div key={group}>
            <div className="mb-1 border-t border-hairline pt-2 text-sm font-semibold text-ink-soft">
              {group}
            </div>
            <ul className="flex flex-col">
              {groupEntries.map((entry) => {
                const row = heldByCode.get(entry.code);
                const label = catalogByCode.get(entry.code)?.label ?? entry.label;
                const stagedRemoval = row ? removals.includes(row.id) : false;
                const stagedAddition = entry.code in additions;
                const isCovered =
                  !row && !stagedAddition && covered.has(entry.code);
                const checked =
                  (!!row && !stagedRemoval) || stagedAddition || isCovered;
                return (
                  <li key={entry.code}>
                    <div className="flex flex-wrap items-center gap-2">
                      <label
                        className={`flex min-h-11 items-center gap-3 text-[15px] ${
                          isCovered ? "text-ink-soft" : ""
                        }`}
                      >
                        <input
                          type="checkbox"
                          className="size-5 accent-abysse-600"
                          checked={checked}
                          disabled={isCovered || busy}
                          onChange={() => toggle(entry.code)}
                        />
                        {label}
                      </label>
                      {row && !stagedRemoval && (
                        <Badge tone={PROVENANCE_TONE[row.provenance] ?? "neutral"}>
                          {t.diving.provenance[row.provenance] ?? row.provenance}
                        </Badge>
                      )}
                      {row?.obtained_on && !stagedRemoval && (
                        <span className="text-xs text-ink-soft">
                          {row.obtained_on}
                        </span>
                      )}
                      {stagedRemoval && (
                        <Badge tone="danger">{t.diving.willBeRemoved}</Badge>
                      )}
                      {isCovered && (
                        <span className="text-xs text-ink-soft">
                          {t.diving.coveredBySelection}
                        </span>
                      )}
                    </div>
                    {stagedAddition && (
                      <div className="mb-2 ml-8 max-w-60">
                        <Field label={t.diving.obtainedOn}>
                          <TextInput
                            type="date"
                            value={additions[entry.code]}
                            disabled={busy}
                            onChange={(e) =>
                              setAdditions((current) => ({
                                ...current,
                                [entry.code]: e.target.value,
                              }))
                            }
                          />
                        </Field>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}

      <Button
        disabled={busy || !hasChanges}
        onClick={() =>
          onSave({
            additions: Object.entries(additions).map(
              ([certification, date]) => ({
                certification,
                obtained_on: date === "" ? null : date,
              }),
            ),
            removals,
            validated,
          })
        }
      >
        {t.common.save}
      </Button>
    </div>
  );
}
