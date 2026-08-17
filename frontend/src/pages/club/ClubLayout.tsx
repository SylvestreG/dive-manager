import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import {
  Navigate,
  NavLink,
  Outlet,
  useNavigate,
  useOutletContext,
  useParams,
} from "react-router";

import { Badge, Card, ErrorState, Loading } from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type ClubContext } from "../../lib/api";
import { setLastClubSlug } from "../../lib/prefs";
import { useMe } from "../../lib/session";

export function useClub() {
  return useOutletContext<ClubContext>();
}

export function can(club: ClubContext, permission: string): boolean {
  return club.permissions.includes(permission);
}

/** Index route: administrators land on Members; plain members get a simple
 * welcome card (they have no club-management tabs). */
export function ClubIndex() {
  const club = useClub();
  if (can(club, "club.members.view")) {
    return <Navigate to="members" replace />;
  }
  return (
    <Card>
      <p className="text-sm text-ink-soft">
        {club.my_role} — {club.name}
      </p>
    </Card>
  );
}

export default function ClubLayout() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const { data: me } = useMe();
  const context = useQuery({
    queryKey: ["club", slug],
    queryFn: () => api.club(slug!).context(),
    enabled: !!slug,
    retry: false,
  });

  // Remember the last visited club: the "Mon club" tab targets it.
  const userId = me?.user.id;
  const visitedSlug = context.data?.slug;
  useEffect(() => {
    if (userId && visitedSlug) setLastClubSlug(userId, visitedSlug);
  }, [userId, visitedSlug]);

  if (context.isPending) return <Loading />;
  if (context.isError || !context.data) {
    return <ErrorState onRetry={() => context.refetch()} />;
  }
  const club = context.data;
  const myClubs = me?.memberships ?? [];

  const tabs = [
    {
      to: "members",
      label: t.club.tabs.members,
      show: can(club, "club.members.view"),
    },
    {
      to: "invitations",
      label: t.club.tabs.invitations,
      show: can(club, "club.invitations.manage"),
    },
    {
      to: "roles",
      label: t.club.tabs.roles,
      show: can(club, "club.roles.manage"),
    },
    {
      to: "modules",
      label: t.club.tabs.modules,
      show: can(club, "club.manage"),
    },
    { to: "audit", label: t.club.tabs.audit, show: can(club, "club.manage") },
  ].filter((tab) => tab.show);

  return (
    <>
      <div className="mb-1 flex flex-wrap items-center gap-2">
        {myClubs.length > 1 ? (
          // Multi-club members switch clubs right here.
          <select
            aria-label={t.club.switch}
            className="min-h-11 rounded-(--radius-control) border border-hairline bg-white px-2 text-lg font-bold"
            value={club.slug}
            onChange={(e) => navigate(`/clubs/${e.target.value}`)}
          >
            {myClubs.map((m) => (
              <option key={m.club_id} value={m.club_slug}>
                {m.club_name}
              </option>
            ))}
            {!myClubs.some((m) => m.club_slug === club.slug) && (
              <option value={club.slug}>{club.name}</option>
            )}
          </select>
        ) : (
          <h1 className="text-xl font-bold">{club.name}</h1>
        )}
        {club.my_role && <Badge tone="primary">{club.my_role}</Badge>}
        {club.via_global_admin && <Badge tone="warning">Admin global</Badge>}
      </div>
      {tabs.length > 0 && (
        <nav className="-mx-4 mb-4 flex gap-1 overflow-x-auto border-b border-hairline px-4">
          {tabs.map((tab) => (
            <NavLink
              key={tab.to}
              to={tab.to}
              className={({ isActive }) =>
                `min-h-11 shrink-0 border-b-2 px-3 pt-2 text-sm font-semibold ${
                  isActive
                    ? "border-abysse-600 text-abysse-700"
                    : "border-transparent text-ink-soft"
                }`
              }
            >
              {tab.label}
            </NavLink>
          ))}
        </nav>
      )}
      <Outlet context={club} />
    </>
  );
}
