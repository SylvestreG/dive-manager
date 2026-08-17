import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import {
  Card,
  EmptyState,
  ErrorState,
  Loading,
  Pager,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api } from "../../lib/api";
import { useClub } from "./ClubLayout";

export default function Audit() {
  const club = useClub();
  const [page, setPage] = useState(1);
  const audit = useQuery({
    queryKey: ["club", club.slug, "audit", page],
    queryFn: () => api.club(club.slug).audit(page),
    placeholderData: keepPreviousData,
  });

  if (audit.isPending) return <Loading />;
  if (audit.isError) return <ErrorState onRetry={() => audit.refetch()} />;
  if (audit.data.items.length === 0) return <EmptyState />;

  return (
    <div className="flex flex-col gap-2">
      <p className="text-sm text-ink-soft">{t.common.total(audit.data.total)}</p>
      {audit.data.items.map((entry) => (
        <Card key={entry.id} className="text-sm">
          <div className="flex flex-wrap items-center justify-between gap-1">
            <span className="font-mono font-semibold text-abysse-700">
              {entry.action}
            </span>
            <span className="text-xs text-ink-soft">
              {new Date(entry.created_at).toLocaleString("fr-FR")}
            </span>
          </div>
          {Object.keys(entry.details).length > 0 && (
            <pre className="mt-1 overflow-x-auto rounded bg-mist p-2 text-xs text-ink-soft">
              {JSON.stringify(entry.details, null, 1)}
            </pre>
          )}
        </Card>
      ))}
      <Pager
        page={page}
        perPage={audit.data.per_page}
        total={audit.data.total}
        onPage={setPage}
      />
    </div>
  );
}
