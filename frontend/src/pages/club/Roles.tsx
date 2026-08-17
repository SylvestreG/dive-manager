import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  Badge,
  Button,
  Card,
  ErrorState,
  FormFeedback,
  Loading,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type RoleDto } from "../../lib/api";
import { CLUB_PERMISSIONS } from "../../lib/permissions";
import { translateApiError } from "../../lib/errors";
import { useClub } from "./ClubLayout";

const ALL_PERMISSIONS = CLUB_PERMISSIONS;

function RoleCard({ role, slug }: { role: RoleDto; slug: string }) {
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<string[]>(role.permissions);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const dirty =
    selected.length !== role.permissions.length ||
    selected.some((p) => !role.permissions.includes(p));

  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.club(slug).setRolePermissions(role.id, selected),
    onSuccess: () => {
      setError(null);
      setSuccess(t.club.roles.saved);
      queryClient.invalidateQueries({ queryKey: ["club", slug, "roles"] });
    },
    onError: (e) => {
      setSuccess(null);
      setError(translateApiError(e));
    },
  });

  const toggle = (permission: string) =>
    setSelected((current) =>
      current.includes(permission)
        ? current.filter((p) => p !== permission)
        : [...current, permission],
    );

  return (
    <Card>
      <div className="flex items-center gap-2">
        <span className="font-bold">{role.name}</span>
        {role.is_admin && (
          <Badge tone="primary">{t.club.roles.adminImplicit}</Badge>
        )}
      </div>
      {!role.is_admin && (
        <>
          <div className="mt-3 flex flex-col gap-2">
            {ALL_PERMISSIONS.map((permission) => (
              <label
                key={permission}
                className="flex min-h-11 items-center gap-3 text-sm"
              >
                <input
                  type="checkbox"
                  className="size-5 accent-abysse-600"
                  checked={selected.includes(permission)}
                  onChange={() => toggle(permission)}
                />
                {t.permissions[permission] ?? permission}
              </label>
            ))}
          </div>
          <Button
            className="mt-2"
            variant="secondary"
            disabled={!dirty || save.isPending}
            onClick={() => save.mutate()}
          >
            {t.club.roles.savePermissions}
          </Button>
          <FormFeedback error={error} success={success} />
        </>
      )}
    </Card>
  );
}

export default function Roles() {
  const club = useClub();
  const roles = useQuery({
    queryKey: ["club", club.slug, "roles"],
    queryFn: () => api.club(club.slug).roles(),
  });

  if (roles.isPending) return <Loading />;
  if (roles.isError) return <ErrorState onRetry={() => roles.refetch()} />;

  return (
    <div className="flex flex-col gap-3">
      {roles.data.map((role) => (
        <RoleCard key={role.id} role={role} slug={club.slug} />
      ))}
    </div>
  );
}
