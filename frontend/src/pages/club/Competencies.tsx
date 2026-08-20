// Club tab "Compétences": pick a prepared level, list the club's active
// members preparing it with their progress, open a member's sheet.
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";

import { ProgressBar } from "../../components/Competencies";
import {
  Card,
  EmptyState,
  ErrorState,
  Field,
  Loading,
  Pager,
  Select,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api } from "../../lib/api";
import { useClub } from "./ClubLayout";

export default function Competencies() {
  const club = useClub();
  const [level, setLevel] = useState<string | null>(null);
  const [page, setPage] = useState(1);

  const levels = useQuery({
    queryKey: ["competencies", "levels"],
    queryFn: () => api.competencies.levels(),
    staleTime: 5 * 60 * 1000,
  });
  // Levels with a defined tree only — nothing to validate elsewhere.
  const treeLevels = (levels.data ?? []).filter((l) => l.domains_count > 0);
  const activeLevel = level ?? treeLevels[0]?.certification ?? null;

  const students = useQuery({
    queryKey: ["club", club.slug, "competency-students", activeLevel, page],
    queryFn: () => api.club(club.slug).competencyStudents(activeLevel!, page),
    enabled: activeLevel != null,
  });

  if (levels.isPending) return <Loading />;
  if (levels.isError) return <ErrorState onRetry={() => levels.refetch()} />;
  if (treeLevels.length === 0) {
    return <EmptyState message={t.competencies.emptyNoTree} />;
  }

  return (
    <>
      <Card className="mb-4">
        <Field label={t.competencies.studentsLevel}>
          <Select
            value={activeLevel ?? ""}
            onChange={(e) => {
              setLevel(e.target.value);
              setPage(1);
            }}
          >
            {treeLevels.map((l) => (
              <option key={l.certification} value={l.certification}>
                {l.label}
              </option>
            ))}
          </Select>
        </Field>
        {students.data && (
          <p className="mt-2 text-xs text-ink-soft">
            {t.competencies.requiredGrade(
              students.data.level.required_validator_grade,
            )}
          </p>
        )}
      </Card>

      {students.isPending && <Loading />}
      {students.isError && (
        <ErrorState onRetry={() => students.refetch()} />
      )}
      {students.data && students.data.items.length === 0 && (
        <EmptyState message={t.competencies.noStudents} />
      )}
      {students.data && students.data.items.length > 0 && (
        <>
          <div className="space-y-2">
            {students.data.items.map((student) => (
              <Link
                key={student.user.id}
                to={`../members/${student.user.id}/competencies?certification=${encodeURIComponent(activeLevel!)}`}
                className="block"
              >
                <Card className="transition-colors hover:bg-abysse-50">
                  <div className="mb-1 flex items-center justify-between gap-3">
                    <span className="text-sm font-semibold">
                      {student.user.first_name} {student.user.last_name}
                    </span>
                    <span className="text-xs font-semibold text-ink-soft">
                      {student.progress.percentage}%
                    </span>
                  </div>
                  <ProgressBar progress={student.progress} />
                  <p className="mt-1 text-xs text-ink-soft">
                    {t.competencies.progressDetail(
                      student.progress.validated,
                      student.progress.in_progress,
                      student.progress.not_started,
                    )}
                  </p>
                </Card>
              </Link>
            ))}
          </div>
          <Pager
            page={students.data.page}
            perPage={students.data.per_page}
            total={students.data.total}
            onPage={setPage}
          />
        </>
      )}
    </>
  );
}
