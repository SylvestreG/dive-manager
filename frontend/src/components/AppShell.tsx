// Authenticated layout: top bar, impersonation banner, content, and a
// bottom tab bar on phones that becomes a left sidebar on md+ screens.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Link,
  Navigate,
  NavLink,
  Outlet,
  useLocation,
  useNavigate,
} from "react-router";

import { t } from "../i18n/fr";
import { api, setToken, type MeResponse } from "../lib/api";
import { translateApiError } from "../lib/errors";
import { getLastClubSlug } from "../lib/prefs";
import { useMe } from "../lib/session";
import { pushErrorToast } from "./toasts";
import { Loading } from "./ui";

function ImpersonationBanner({ me }: { me: MeResponse }) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const stop = async () => {
    try {
      const session = await api.stopImpersonation();
      setToken(session.token);
      // Principal changed: drop every cached response and land on home so
      // the whole UI remounts with the admin's own credentials (same
      // pattern as entering impersonation).
      queryClient.clear();
      navigate("/", { replace: true });
    } catch (e) {
      pushErrorToast(translateApiError(e));
    }
  };
  if (!me.impersonating) return null;
  return (
    <div className="flex items-center justify-between gap-3 bg-warning-600 px-4 py-2 text-sm font-semibold text-white">
      <span>
        {t.impersonation.banner(
          `${me.user.first_name} ${me.user.last_name}`.trim(),
        )}
      </span>
      <button
        onClick={stop}
        className="min-h-9 shrink-0 rounded-(--radius-control) bg-white/20 px-3 font-semibold hover:bg-white/30"
      >
        {t.impersonation.stop}
      </button>
    </div>
  );
}

/** Red banner shown app-wide while the diving profile has no birthdate
 * (product decision 2026-08-17) — hidden on the profile page itself. */
function IncompleteProfileBanner() {
  const location = useLocation();
  const profile = useQuery({
    queryKey: ["me", "profile"],
    queryFn: () => api.myProfile(),
    // Banner flag only: no need to refetch on every mount/focus. Mutations
    // update the ["me", "profile"] cache directly.
    staleTime: 5 * 60 * 1000,
  });
  if (
    location.pathname === "/profile" ||
    !profile.data ||
    !profile.data.derived.incomplete
  ) {
    return null;
  }
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 bg-danger-600 px-4 py-2 text-sm font-semibold text-white">
      <span>{t.diving.incompleteBanner}</span>
      <Link
        to="/profile"
        className="min-h-9 shrink-0 content-center rounded-(--radius-control) bg-white/20 px-3 font-semibold hover:bg-white/30"
      >
        {t.diving.incompleteBannerCta}
      </Link>
    </div>
  );
}

function tabClass({ isActive }: { isActive: boolean }) {
  return `flex min-h-12 flex-1 flex-col items-center justify-center gap-0.5 text-xs font-semibold md:min-h-11 md:flex-none md:flex-row md:justify-start md:gap-2 md:rounded-(--radius-control) md:px-3 md:text-sm ${
    isActive
      ? "text-abysse-700 md:bg-abysse-50"
      : "text-ink-soft hover:text-abysse-700"
  }`;
}

export default function AppShell() {
  const { data: me, isPending, isError } = useMe();
  const location = useLocation();

  if (isPending) return <Loading />;
  if (isError || !me) {
    return <Navigate to="/login" replace />;
  }

  // "Mon club" targets, in order: the club currently on screen, else the last
  // visited club, else the first membership. Multi-club users also switch
  // from the club-header selector and the home cards.
  const pathClub = location.pathname.match(/^\/clubs\/([^/]+)/)?.[1];
  const lastClub = getLastClubSlug(me.user.id);
  const defaultClub =
    me.memberships.find((m) => m.club_slug === pathClub)?.club_slug ??
    me.memberships.find((m) => m.club_slug === lastClub)?.club_slug ??
    me.memberships[0]?.club_slug;
  const tabs = [
    { to: "/", label: t.nav.home, icon: "⌂", end: true },
    ...(defaultClub
      ? [{ to: `/clubs/${defaultClub}`, label: t.nav.club, icon: "🤿", end: false }]
      : []),
    { to: "/competencies", label: t.nav.competencies, icon: "🎓", end: false },
    // Platform tab: real global admins only — and hidden while impersonating
    // (platform endpoints refuse impersonating principals anyway).
    ...(!me.impersonating && me.user.is_global_admin
      ? [{ to: "/platform", label: t.nav.platform, icon: "⚙", end: false }]
      : []),
    { to: "/profile", label: t.nav.profile, icon: "👤", end: false },
  ];

  return (
    <div className="min-h-dvh md:flex">
      <div className="sticky top-0 z-20 md:hidden">
        <ImpersonationBanner me={me} />
        <IncompleteProfileBanner />
        <header className="border-b border-hairline bg-white px-4 py-3">
          <span className="text-base font-bold text-abysse-700">
            {t.appName}
          </span>
        </header>
      </div>

      {/* Sidebar (md+) */}
      <aside className="hidden w-56 shrink-0 border-r border-hairline bg-white p-4 md:block">
        <div className="mb-6 text-lg font-bold text-abysse-700">{t.appName}</div>
        <nav className="flex flex-col gap-1">
          {tabs.map((tab) => (
            <NavLink key={tab.to} to={tab.to} end={tab.end} className={tabClass}>
              <span aria-hidden>{tab.icon}</span>
              {tab.label}
            </NavLink>
          ))}
        </nav>
      </aside>

      <div className="flex-1">
        <div className="hidden md:block">
          <ImpersonationBanner me={me} />
          <IncompleteProfileBanner />
        </div>
        <main className="mx-auto max-w-4xl px-4 pb-24 pt-4 md:pb-8">
          <Outlet />
        </main>
      </div>

      {/* Bottom tab bar (phones) */}
      <nav
        className="fixed inset-x-0 bottom-0 z-20 flex border-t border-hairline bg-white md:hidden"
        style={{ paddingBottom: "env(safe-area-inset-bottom)" }}
      >
        {tabs.map((tab) => (
          <NavLink key={tab.to} to={tab.to} end={tab.end} className={tabClass}>
            <span aria-hidden className="text-lg leading-none">
              {tab.icon}
            </span>
            {tab.label}
          </NavLink>
        ))}
      </nav>
    </div>
  );
}
