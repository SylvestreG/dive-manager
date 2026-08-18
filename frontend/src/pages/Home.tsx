import { Link, Navigate } from "react-router";

import { Badge, Card, EmptyState, PageTitle } from "../components/ui";
import { t } from "../i18n/fr";
import { useMe } from "../lib/session";

export default function Home() {
  const { data: me } = useMe();
  if (!me) return null;

  // Product decision 2026-08-18: users with exactly one club land directly in it, no club picker.
  if (me.memberships.length === 1) {
    return <Navigate to={`/clubs/${me.memberships[0].club_slug}`} replace />;
  }

  return (
    <>
      <PageTitle>{t.home.hello(me.user.first_name)}</PageTitle>
      <h2 className="mb-2 text-base font-bold">{t.home.myClubs}</h2>
      {me.memberships.length === 0 ? (
        <EmptyState message={t.home.noClub} />
      ) : (
        <div className="flex flex-col gap-3">
          {me.memberships.map((membership) => (
            <Link key={membership.club_id} to={`/clubs/${membership.club_slug}`}>
              <Card className="flex items-center justify-between gap-3 transition-colors hover:border-abysse-600">
                <div>
                  <div className="font-bold">{membership.club_name}</div>
                  <div className="mt-1">
                    <Badge tone={membership.is_club_admin ? "primary" : "neutral"}>
                      {membership.role_name}
                    </Badge>
                  </div>
                </div>
                <span className="font-semibold text-abysse-700">
                  {t.home.open} →
                </span>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </>
  );
}
