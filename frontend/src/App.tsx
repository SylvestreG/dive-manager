import { Navigate, Outlet, Route, Routes } from "react-router";

import AppShell from "./components/AppShell";
import { getToken } from "./lib/api";
import { useUnauthorizedRedirect } from "./lib/session";
import AcceptInvitation from "./pages/AcceptInvitation";
import Home from "./pages/Home";
import Login from "./pages/Login";
import MagicLink from "./pages/MagicLink";
import Platform from "./pages/Platform";
import Profile from "./pages/Profile";
import Audit from "./pages/club/Audit";
import ClubLayout, { ClubIndex } from "./pages/club/ClubLayout";
import Invitations from "./pages/club/Invitations";
import MemberProfile from "./pages/club/MemberProfile";
import Members from "./pages/club/Members";
import Modules from "./pages/club/Modules";

function RequireAuth() {
  if (!getToken()) return <Navigate to="/login" replace />;
  return <Outlet />;
}

export default function App() {
  useUnauthorizedRedirect();
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      {/* These two paths are the links the backend emails out. */}
      <Route path="/auth/ml/:token" element={<MagicLink />} />
      <Route path="/auth/invitations/:token" element={<AcceptInvitation />} />

      <Route element={<RequireAuth />}>
        <Route element={<AppShell />}>
          <Route index element={<Home />} />
          <Route path="/platform" element={<Platform />} />
          <Route path="/profile" element={<Profile />} />
          <Route path="/clubs/:slug" element={<ClubLayout />}>
            <Route index element={<ClubIndex />} />
            <Route path="members" element={<Members />} />
            <Route path="members/:userId" element={<MemberProfile />} />
            <Route path="invitations" element={<Invitations />} />
            <Route path="modules" element={<Modules />} />
            <Route path="audit" element={<Audit />} />
          </Route>
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
