// Shared competency rendering: fixed-stage badges, weighted progress bars,
// legend, and the domain → module → skill tree. Read-only by default; the
// club validation sheet passes `onStagePick` to turn skills interactive.
import type { ReactNode } from "react";

import { t } from "../i18n/fr";
import type {
  CompetencyProgress,
  CompetencyView,
  LevelRules,
  SkillView,
  StageCode,
} from "../lib/api";
import { Badge, Card } from "./ui";

/** Visual identity of the fixed ladder (backend enforces the semantics). */
export const STAGE_ORDER: StageCode[] = ["vu", "a_perfectionner", "valide"];
const STAGE_TONES: Record<StageCode, "primary" | "warning" | "success"> = {
  vu: "primary",
  a_perfectionner: "warning",
  valide: "success",
};

export function stageLabel(stage: StageCode, level: LevelRules): string {
  return t.competencies.stageOf(stage, level.sea_validation);
}

export function StageBadge({
  stage,
  level,
}: {
  stage: StageCode | null;
  level: LevelRules;
}) {
  if (!stage) {
    return <Badge tone="neutral">{t.competencies.stages.not_started}</Badge>;
  }
  return <Badge tone={STAGE_TONES[stage]}>{stageLabel(stage, level)}</Badge>;
}

export function StageLegend({ level }: { level: LevelRules }) {
  return (
    <div>
      <p className="mb-1 text-xs font-semibold text-ink-soft">
        {t.competencies.legend}
      </p>
      <div className="flex flex-wrap items-center gap-1.5">
        {level.stages.map((stage) => (
          <StageBadge key={stage} stage={stage} level={level} />
        ))}
        <StageBadge stage={null} level={level} />
      </div>
      {level.sea_validation && (
        <p className="mt-1 text-xs text-ink-soft">{t.competencies.seaRule}</p>
      )}
    </div>
  );
}

/** Two-segment bar: green = validated share, amber = in-progress share. */
export function ProgressBar({ progress }: { progress: CompetencyProgress }) {
  const share = (n: number) =>
    progress.total === 0 ? 0 : (n / progress.total) * 100;
  return (
    <div
      className="h-2 w-full overflow-hidden rounded-full bg-mist"
      role="progressbar"
      aria-valuenow={progress.percentage}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div className="flex h-full">
        <div
          className="bg-success-600"
          style={{ width: `${share(progress.validated)}%` }}
        />
        <div
          className="bg-warning-600"
          style={{ width: `${share(progress.in_progress)}%` }}
        />
      </div>
    </div>
  );
}

export function ProgressSummary({
  progress,
}: {
  progress: CompetencyProgress;
}) {
  return (
    <div>
      <div className="mb-1 flex items-baseline justify-between gap-2">
        <span className="text-2xl font-bold">{progress.percentage}%</span>
        <span className="text-sm text-ink-soft">
          {t.competencies.progress(progress.validated, progress.total)}
        </span>
      </div>
      <ProgressBar progress={progress} />
      <p className="mt-1 text-xs text-ink-soft">
        {t.competencies.progressDetail(
          progress.validated,
          progress.in_progress,
          progress.not_started,
        )}
      </p>
    </div>
  );
}

function SkillRow({
  skill,
  level,
  renderActions,
}: {
  skill: SkillView;
  level: LevelRules;
  renderActions?: (skill: SkillView) => ReactNode;
}) {
  const validation = skill.validation;
  return (
    <li className="border-t border-hairline py-2 first:border-t-0">
      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
        <span className="text-sm font-medium">{skill.name}</span>
        <StageBadge stage={validation?.stage ?? null} level={level} />
      </div>
      {skill.description && (
        <p className="mt-0.5 text-xs italic text-ink-soft">
          💡 {skill.description}
        </p>
      )}
      {validation && (
        <p className="mt-0.5 text-xs text-ink-soft">
          {t.competencies.validatedOn(
            new Date(validation.validated_at).toLocaleDateString("fr-FR"),
          )}
          {validation.validated_by &&
            ` ${t.competencies.validatedBy(
              `${validation.validated_by.first_name} ${validation.validated_by.last_name}`,
            )}`}
          {validation.notes && (
            <span className="italic"> — {validation.notes}</span>
          )}
        </p>
      )}
      {renderActions?.(skill)}
    </li>
  );
}

/** The full tree of one level for one person. */
export function CompetencyTreeView({
  view,
  renderSkillActions,
}: {
  view: CompetencyView;
  renderSkillActions?: (skill: SkillView) => ReactNode;
}) {
  if (view.domains.length === 0) {
    return (
      <Card className="text-center text-sm text-ink-soft">
        {t.competencies.emptyNoTree}
      </Card>
    );
  }
  return (
    <div className="space-y-4">
      {view.domains.map((domain) => (
        <Card key={domain.id}>
          <div className="mb-2 flex items-center justify-between gap-3">
            <h3 className="text-sm font-bold uppercase tracking-wide text-abysse-700">
              {domain.name}
            </h3>
            <span className="text-xs font-semibold text-ink-soft">
              {domain.progress.percentage}%
            </span>
          </div>
          <ProgressBar progress={domain.progress} />
          <div className="mt-3 space-y-3">
            {domain.modules.map((module) => (
              <div key={module.id}>
                <div className="flex items-center justify-between gap-3">
                  <h4 className="text-sm font-semibold">{module.name}</h4>
                  <span className="text-xs text-ink-soft">
                    {t.competencies.progress(
                      module.progress.validated,
                      module.progress.total,
                    )}
                  </span>
                </div>
                <ul>
                  {module.skills.map((skill) => (
                    <SkillRow
                      key={skill.id}
                      skill={skill}
                      level={view.level}
                      renderActions={renderSkillActions}
                    />
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </Card>
      ))}
    </div>
  );
}
