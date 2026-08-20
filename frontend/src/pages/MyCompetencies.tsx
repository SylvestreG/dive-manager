// Member view "Mes compétences": progress on the prepared level, the full
// tree (read-only) and the filled level document download. Store-v1 scope.
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import {
  CompetencyTreeView,
  ProgressSummary,
  StageLegend,
} from "../components/Competencies";
import { pushErrorToast } from "../components/toasts";
import {
  Button,
  Card,
  EmptyState,
  ErrorState,
  Loading,
  PageTitle,
} from "../components/ui";
import { t } from "../i18n/fr";
import { api, ApiError } from "../lib/api";
import { translateApiError } from "../lib/errors";

export default function MyCompetencies() {
  const [downloading, setDownloading] = useState(false);
  const view = useQuery({
    queryKey: ["me", "competencies"],
    queryFn: () => api.competencies.my(),
    retry: false,
  });
  const levels = useQuery({
    queryKey: ["competencies", "levels"],
    queryFn: () => api.competencies.levels(),
    staleTime: 5 * 60 * 1000,
  });

  if (view.isPending) return <Loading />;
  if (view.isError) {
    // 422 = nothing in preparation: a normal state, not an error.
    if (view.error instanceof ApiError && view.error.status === 422) {
      return (
        <>
          <PageTitle>{t.competencies.myTitle}</PageTitle>
          <EmptyState message={t.competencies.emptyNoPreparation} />
          <Card className="mt-4">
            <h2 className="mb-1 text-sm font-bold">
              {t.competencies.explainerTitle}
            </h2>
            <p className="text-sm text-ink-soft">{t.competencies.explainer}</p>
          </Card>
        </>
      );
    }
    return <ErrorState onRetry={() => view.refetch()} />;
  }
  const data = view.data;
  const hasDocument = levels.data?.some(
    (l) => l.certification === data.level.certification && l.has_document,
  );

  const downloadPdf = async () => {
    setDownloading(true);
    try {
      const blob = await api.competencies.myDocument(data.level.certification);
      if (!blob) {
        pushErrorToast(translateApiError(new ApiError(404, "not_found", "no document for this level")));
        return;
      }
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `Competences_${data.level.certification}.pdf`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      pushErrorToast(translateApiError(e));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <>
      <PageTitle>{t.competencies.myTitle}</PageTitle>
      <Card className="mb-4">
        <p className="mb-2 text-sm font-semibold text-abysse-700">
          {t.competencies.preparing(data.level.label)}
        </p>
        <ProgressSummary progress={data.progress} />
        <div className="mt-3">
          <StageLegend level={data.level} />
        </div>
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
      <CompetencyTreeView view={data} />
      <Card className="mt-4">
        <h2 className="mb-1 text-sm font-bold">
          {t.competencies.explainerTitle}
        </h2>
        <p className="text-sm text-ink-soft">{t.competencies.explainer}</p>
      </Card>
    </>
  );
}
