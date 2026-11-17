import { lazy, Suspense } from "react";
import { Navigate, Outlet, Route, Routes } from "react-router";

import AppShell from "./components/AppShell";
import { getToken } from "./lib/api";
import { useUnauthorizedRedirect } from "./lib/session";
import Home from "./pages/Home";
import AuthDone, { AuthNotice } from "./pages/AuthDone";
import Login from "./pages/Login";
import Privacy from "./pages/Privacy";
import MagicLink from "./pages/MagicLink";
import MyCompetencies from "./pages/MyCompetencies";
import Platform from "./pages/Platform";
import Profile from "./pages/Profile";
import Audit from "./pages/club/Audit";
import ClubLayout, { ClubIndex } from "./pages/club/ClubLayout";
import Competencies from "./pages/club/Competencies";
import MemberCompetencies from "./pages/club/MemberCompetencies";
import MemberProfile from "./pages/club/MemberProfile";
import Members from "./pages/club/Members";
import CompetenciesAdmin from "./pages/platform/CompetenciesAdmin";
import { Loading } from "./components/ui";

// Desktop-only admin editor: pdf.js (~1 MB) stays out of the member bundle.
const LevelDocuments = lazy(() => import("./pages/platform/LevelDocuments"));

function RequireAuth() {
  if (!getToken()) return <Navigate to="/login" replace />;
  return <Outlet />;
}

export default function App() {
  useUnauthorizedRedirect();
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/confidentialite" element={<Privacy />} />
      {/* SSO landing pages (backend callback redirects here). */}
      <Route path="/auth/done" element={<AuthDone />} />
      <Route path="/auth/check-email" element={<AuthNotice kind="check-email" />} />
      <Route path="/auth/unknown" element={<AuthNotice kind="unknown" />} />
      {/* These two paths are the links the backend emails out. */}
      <Route path="/auth/ml/:token" element={<MagicLink />} />

      <Route element={<RequireAuth />}>
        <Route element={<AppShell />}>
          <Route index element={<Home />} />
          <Route path="/competencies" element={<MyCompetencies />} />
          <Route path="/platform" element={<Platform />} />
          <Route path="/platform/competencies" element={<CompetenciesAdmin />} />
          <Route
            path="/platform/level-documents"
            element={
              <Suspense fallback={<Loading />}>
                <LevelDocuments />
              </Suspense>
            }
          />
          <Route path="/profile" element={<Profile />} />
          <Route path="/clubs/:slug" element={<ClubLayout />}>
            <Route index element={<ClubIndex />} />
            <Route path="members" element={<Members />} />
            <Route path="members/:userId" element={<MemberProfile />} />
            <Route
              path="members/:userId/competencies"
              element={<MemberCompetencies />}
            />
            <Route path="competencies" element={<Competencies />} />
            {/* No Modules tab (product decision 2026-08-19: core/foundation
                only for now — the tenancy registry stays server-side). */}
            <Route path="audit" element={<Audit />} />
          </Route>
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
