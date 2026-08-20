// Member competency sheet: the tree with stage buttons for qualified
// validators. Forward-only progression with visibly disabled backward
// stages; deletion reserved to real global admins; PDF generation for staff.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useParams, useSearchParams } from "react-router";

import {
  STAGE_ORDER,
  StageLegend,
  ProgressSummary,
  CompetencyTreeView,
  stageLabel,
} from "../../components/Competencies";
import { pushErrorToast } from "../../components/toasts";
import {
  Button,
  Card,
  ErrorState,
  Loading,
  PageTitle,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type SkillView, type StageCode } from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { useMe } from "../../lib/session";
import { useClub } from "./ClubLayout";

/** Client-side mirror of the backend grade gate (pre-disables the buttons;
 * the server re-checks on every write). */
function gradeAtLeast(held: string | null, required: string): boolean {
  const rank = (g: string) => ["E1", "E2", "E3", "E4"].indexOf(g);
  return held != null && rank(held) >= rank(required);
}

export default function MemberCompetencies() {
  const club = useClub();
  const { userId } = useParams<{ userId: string }>();
  const [search] = useSearchParams();
  const certification = search.get("certification") ?? "";
  const queryClient = useQueryClient();
  const { data: me } = useMe();
  const [downloading, setDownloading] = useState(false);

  const view = useQuery({
    queryKey: ["club", club.slug, "member-competencies", userId, certification],
    queryFn: () =>
      api.club(club.slug).memberCompetencies(userId!, certification),
    enabled: !!userId && !!certification,
  });
  const profile = useQuery({
    queryKey: ["club", club.slug, "member", userId],
    queryFn: () => api.club(club.slug).memberProfile(userId!),
    enabled: !!userId,
  });
  const myProfile = useQuery({
    queryKey: ["me", "profile"],
    queryFn: () => api.myProfile(),
    staleTime: 5 * 60 * 1000,
  });
  const levels = useQuery({
    queryKey: ["competencies", "levels"],
    queryFn: () => api.competencies.levels(),
    staleTime: 5 * 60 * 1000,
  });

  const validate = useMutation({
    mutationFn: ({ skillId, stage }: { skillId: string; stage: StageCode }) =>
      api.club(club.slug).putSkillValidation(userId!, skillId, { stage }),
    onSuccess: (updated) => {
      queryClient.setQueryData(
        ["club", club.slug, "member-competencies", userId, certification],
        updated,
      );
      queryClient.invalidateQueries({
        queryKey: ["club", club.slug, "competency-students"],
      });
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
    meta: { inlineError: true },
  });
  const remove = useMutation({
    mutationFn: (skillId: string) =>
      api.club(club.slug).deleteSkillValidation(userId!, skillId),
    onSuccess: (updated) => {
      queryClient.setQueryData(
        ["club", club.slug, "member-competencies", userId, certification],
        updated,
      );
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
    meta: { inlineError: true },
  });

  if (view.isPending || profile.isPending) return <Loading />;
  if (view.isError || profile.isError || !view.data || !profile.data) {
    return <ErrorState onRetry={() => view.refetch()} />;
  }
  const data = view.data;
  const level = data.level;
  const memberName =
    `${profile.data.user.first_name} ${profile.data.user.last_name}`.trim();
  const impersonating = me?.impersonating ?? false;
  const isRealGlobalAdmin = !impersonating && (me?.user.is_global_admin ?? false);
  const canValidate =
    !impersonating &&
    me?.user.id !== userId &&
    (isRealGlobalAdmin ||
      gradeAtLeast(
        myProfile.data?.derived.teaching_certified ?? null,
        level.required_validator_grade,
      ));
  const hasDocument = levels.data?.some(
    (l) => l.certification === level.certification && l.has_document,
  );

  const pickStage = (skill: SkillView, stage: StageCode) => {
    const label = stageLabel(stage, level);
    const lines = [
      t.competencies.validateConfirm(skill.name, label),
      t.competencies.validateIrreversible,
    ];
    if (stage === "valide" && level.sea_validation) {
      lines.push(t.competencies.validateSeaReminder);
    }
    if (window.confirm(lines.join("\n\n"))) {
      validate.mutate({ skillId: skill.id, stage });
    }
  };

  const downloadPdf = async () => {
    setDownloading(true);
    try {
      const blob = await api
        .club(club.slug)
        .memberCompetenciesDocument(userId!, level.certification);
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `Competences_${level.certification}_${memberName}.pdf`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      pushErrorToast(translateApiError(e));
    } finally {
      setDownloading(false);
    }
  };

  const skillActions = (skill: SkillView) => {
    if (!canValidate) return null;
    const currentRank = skill.validation
      ? STAGE_ORDER.indexOf(skill.validation.stage)
      : -1;
    return (
      <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
        {level.stages.map((stage) => {
          const backward =
            STAGE_ORDER.indexOf(stage) < currentRank && !isRealGlobalAdmin;
          return (
            <button
              key={stage}
              disabled={backward || validate.isPending}
              title={backward ? t.competencies.validateIrreversible : undefined}
              onClick={() => pickStage(skill, stage)}
              className={`min-h-9 rounded-(--radius-control) border px-2.5 text-xs font-semibold transition-colors ${
                skill.validation?.stage === stage
                  ? "border-abysse-600 bg-abysse-600 text-white"
                  : backward
                    ? "cursor-not-allowed border-hairline text-ink-soft opacity-30"
                    : "border-hairline bg-white text-abysse-700 hover:bg-abysse-50"
              }`}
            >
              {stageLabel(stage, level)}
            </button>
          );
        })}
        {isRealGlobalAdmin && skill.validation && (
          <button
            disabled={remove.isPending}
            onClick={() => {
              if (window.confirm(t.competencies.deleteConfirm(skill.name))) {
                remove.mutate(skill.id);
              }
            }}
            className="min-h-9 rounded-(--radius-control) px-2 text-xs font-semibold text-danger-600 hover:bg-danger-50"
          >
            🗑 {t.competencies.deleteValidation}
          </button>
        )}
      </div>
    );
  };

  return (
    <>
      <PageTitle>{t.competencies.sheetTitle(memberName)}</PageTitle>
      <Card className="mb-4">
        <p className="mb-2 text-sm font-semibold text-abysse-700">
          {t.competencies.preparing(level.label)}
        </p>
        <ProgressSummary progress={data.progress} />
        <div className="mt-3">
          <StageLegend level={level} />
        </div>
        {!canValidate && (
          <p className="mt-2 text-xs text-ink-soft">
            {t.competencies.requiredGrade(level.required_validator_grade)}
          </p>
        )}
        {hasDocument && (
          <Button
            variant="secondary"
            className="mt-3"
            disabled={downloading}
            onClick={downloadPdf}
          >
            📥 {t.competencies.downloadPdf}
          </Button>
        )}
      </Card>
      <CompetencyTreeView view={data} renderSkillActions={skillActions} />
    </>
  );
}
