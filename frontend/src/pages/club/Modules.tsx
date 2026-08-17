import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { Card, ErrorState, Loading } from "../../components/ui";
import { t } from "../../i18n/fr";
import { api } from "../../lib/api";
import { useClub } from "./ClubLayout";

export default function Modules() {
  const club = useClub();
  const queryClient = useQueryClient();
  const key = ["club", club.slug, "modules"];
  const modules = useQuery({
    queryKey: key,
    queryFn: () => api.club(club.slug).modules(),
  });
  const toggle = useMutation({
    mutationFn: ({ module, enabled }: { module: string; enabled: boolean }) =>
      api.club(club.slug).setModule(module, enabled),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: key }),
  });

  if (modules.isPending) return <Loading />;
  if (modules.isError) return <ErrorState onRetry={() => modules.refetch()} />;

  return (
    <>
      <p className="mb-3 text-sm text-ink-soft">{t.club.modules.hint}</p>
      <div className="flex flex-col gap-2">
        {modules.data.map((entry) => (
          <Card key={entry.module}>
            <label className="flex min-h-11 cursor-pointer items-center justify-between gap-3">
              <span className="font-semibold">
                {t.modules[entry.module] ?? entry.module}
              </span>
              <input
                type="checkbox"
                className="size-6 accent-abysse-600"
                checked={entry.enabled}
                disabled={toggle.isPending}
                onChange={() =>
                  toggle.mutate({
                    module: entry.module,
                    enabled: !entry.enabled,
                  })
                }
              />
            </label>
          </Card>
        ))}
      </div>
    </>
  );
}
