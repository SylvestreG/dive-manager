// Typed fetch client for the Dive Manager API.
// Base URL: VITE_API_URL (iOS builds) or same-origin /api/v1 (web dev proxy).

const BASE: string = import.meta.env.VITE_API_URL ?? "/api/v1";
const TOKEN_KEY = "dm_session_token";

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}
export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}
export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = {};
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  if (body !== undefined) headers["Content-Type"] = "application/json";

  let response: Response;
  try {
    response = await fetch(`${BASE}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "network", "network error");
  }

  // A 401 means "invalid session" ONLY on authenticated endpoints. Public
  // auth endpoints also answer 401 (wrong password, expired link) and must
  // not purge a legitimate session state.
  const isPublicAuthPath =
    path === "/auth/login" ||
    path === "/auth/login-methods" ||
    path === "/auth/magic-link" ||
    path === "/auth/magic-link/consume";
  if (response.status === 401 && !isPublicAuthPath) {
    clearToken();
    window.dispatchEvent(new Event("dm:unauthorized"));
  }
  if (!response.ok) {
    let code = "unknown";
    let message = `HTTP ${response.status}`;
    try {
      const payload = await response.json();
      code = payload?.error?.code ?? code;
      message = payload?.error?.message ?? message;
    } catch {
      // keep defaults
    }
    throw new ApiError(response.status, code, message);
  }
  return (await response.json()) as T;
}

/** Authenticated fetch of a binary resource (stamp images). Returns null on
 * 404 (no stamp) so callers can render an empty state without a toast. */
async function requestBlob(path: string): Promise<Blob | null> {
  const headers: Record<string, string> = {};
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  let response: Response;
  try {
    response = await fetch(`${BASE}${path}`, { headers });
  } catch {
    throw new ApiError(0, "network", "network error");
  }
  if (response.status === 404) return null;
  if (response.status === 401) {
    clearToken();
    window.dispatchEvent(new Event("dm:unauthorized"));
  }
  if (!response.ok) {
    throw new ApiError(response.status, "unknown", `HTTP ${response.status}`);
  }
  return await response.blob();
}

// ---------- types (mirror backend DTOs) ----------

export interface UserDto {
  id: string;
  email: string;
  first_name: string;
  last_name: string;
  is_global_admin: boolean;
}
export interface SessionResponse {
  token: string;
  user: UserDto;
}
export interface MeMembership {
  club_id: string;
  club_slug: string;
  club_name: string;
  role_name: string;
  is_club_admin: boolean;
  permissions: string[];
}
export interface MeResponse {
  user: UserDto;
  impersonating: boolean;
  real_user?: UserDto;
  /** Whether the REAL user has a password credential. */
  has_password: boolean;
  memberships: MeMembership[];
}
export interface Page<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}
export interface ClubDto {
  id: string;
  name: string;
  slug: string;
  status: string;
}
export interface ClubContext {
  id: string;
  name: string;
  slug: string;
  my_role: string | null;
  permissions: string[];
  via_global_admin: boolean;
}
export interface MembershipDto {
  id: string;
  user: UserDto;
  role_id: string;
  role_name: string;
  is_admin: boolean;
  status: string;
}
export interface RoleDto {
  id: string;
  name: string;
  is_admin: boolean;
  permissions: string[];
}
export interface InvitationDto {
  id: string;
  email: string;
  first_name: string;
  last_name: string;
  role_id: string | null;
  expires_at: string;
  created_at: string;
}
export interface InvitationPreview {
  email: string;
  first_name: string;
  last_name: string;
  club_name: string | null;
  role_name: string | null;
  expires_at: string;
}
export interface ModuleDto {
  module: string;
  enabled: boolean;
}
export interface AuditEntryDto {
  id: string;
  actor_user_id: string;
  impersonated_user_id: string | null;
  action: string;
  entity_type: string;
  entity_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

// ---- users module (diving profile) ----

export interface ProfileFields {
  birthdate: string | null;
  phone: string | null;
  licence_number: string | null;
  instructor_number: string | null;
  medical_certificate_valid_until: string | null;
  gear_regulator: boolean;
  gear_stab: boolean;
  gear_stab_size: string | null;
  gear_nitrox: boolean;
  gear_second_regulator: boolean;
  preparing_certification: string | null;
  /** Handisub pathway (situation de handicap) — opens PESH suggestions. */
  handisub: boolean;
}
export interface CertificationDto {
  id: string;
  code: string;
  label: string;
  obtained_on: string | null;
  provenance: "declared" | "recorded" | "imported" | "validated";
}
export interface TrainingStatusDto {
  status: string;
  valid_until: string;
}
export interface AptitudeDto {
  code: string;
  kind: "guided" | "autonomous" | "pesh";
  max_depth_m: number;
}
export interface CertRefDto {
  code: string;
  label: string;
}
export interface DerivedDto {
  incomplete: boolean;
  age: number | null;
  aptitudes: AptitudeDto[];
  restrictions: string[];
  gp: boolean;
  teaching: string | null;
  /** From certifications only — opens the members list (derived rule). */
  teaching_certified: string | null;
  dp_pool: boolean;
  dp_exploration: boolean;
  dp_teaching: boolean;
  dp_trimix_shallow: boolean;
  dp_trimix: boolean;
  dp_mixtures_ok: boolean;
  max_o2_percent: number;
  eligible_next: CertRefDto[];
  /** Feeds the preparing picker (unmet entry requirements tolerated). */
  preparable: CertRefDto[];
  preparing_issues: string[];
  medical_expired: boolean | null;
  regulation_snapshot: string;
}
/** Result of importing a member's official FFESSM licence. */
export interface FfessmImportResult {
  /** The member's full profile after the import (drop-in for the query cache). */
  profile: ProfileResponse;
  /** Name printed on the licence — lets staff confirm the right person. */
  holder_name: string | null;
  licence_number: string | null;
  /** Certification codes written/refreshed from the licence. */
  imported: string[];
  /** Codes left untouched because a stronger (validated) row already existed. */
  skipped_stronger: string[];
  /** Brevet titles the licence listed that could not be mapped to a code. */
  unmapped: string[];
}
export interface ProfileResponse {
  user: UserDto;
  profile: ProfileFields;
  certifications: CertificationDto[];
  training_statuses: TrainingStatusDto[];
  derived: DerivedDto;
  has_stamp: boolean;
}
export interface MemberBadges {
  diver: string | null;
  teaching: string | null;
  gp: boolean;
  has_rifap: boolean;
  medical_expired: boolean | null;
  incomplete: boolean;
}
/** Static FFESSM catalog entry: `covers` lists the codes this certification
 * makes acquired (derived server-side, never stored). */
export interface CatalogEntry {
  code: string;
  label: string;
  covers: string[];
}
export interface MemberListItem {
  membership_id: string;
  user: UserDto;
  role_id: string;
  role_name: string;
  is_admin: boolean;
  status: string;
  badges: MemberBadges;
}

// ---------- endpoints ----------

export const api = {
  loginMethods: (email: string) =>
    request<{ password: boolean }>("POST", "/auth/login-methods", { email }),
  requestMagicLink: (email: string) =>
    request<{ message: string }>("POST", "/auth/magic-link", { email }),
  consumeMagicLink: (token: string) =>
    request<SessionResponse>("POST", "/auth/magic-link/consume", { token }),
  loginPassword: (email: string, password: string) =>
    request<SessionResponse>("POST", "/auth/login", { email, password }),
  logout: () => request<unknown>("POST", "/auth/logout"),
  logoutAll: () => request<unknown>("POST", "/auth/logout-all"),
  stopImpersonation: () =>
    request<SessionResponse>("POST", "/auth/stop-impersonation"),
  me: () => request<MeResponse>("GET", "/me"),
  setMyPassword: (password: string) =>
    request<unknown>("PUT", "/me/password", { password }),
  // Own first/last name (bootstrap accounts start with an email-derived
  // name; it also feeds the licence-import identity binding).
  updateMyIdentity: (firstName: string, lastName: string) =>
    request<UserDto>("PUT", "/me/identity", {
      first_name: firstName,
      last_name: lastName,
    }),

  myProfile: () => request<ProfileResponse>("GET", "/me/profile"),
  updateMyProfile: (data: ProfileFields) =>
    request<ProfileResponse>("PUT", "/me/profile", data),
  // Certifications are staff-entered only (see club.recordCertification) —
  // EXCEPT one's own official FFESSM licence (product decision 2026-08-18):
  // scanned QR URL or number+key, fills profile + brevets ("imported").
  importMyFfessmLicence: (body: { url?: string; number?: string; key?: string }) =>
    request<FfessmImportResult>("POST", "/me/ffessm/import", body),
  certificationCatalog: () =>
    request<CatalogEntry[]>("GET", "/catalog/certifications"),
  myStamp: () => requestBlob("/me/stamp"),
  putMyStamp: (imageBase64: string) =>
    request<unknown>("PUT", "/me/stamp", { image_base64: imageBase64 }),
  deleteMyStamp: () => request<unknown>("DELETE", "/me/stamp"),

  previewInvitation: (token: string) =>
    request<InvitationPreview>("GET", `/auth/invitations/${token}`),
  acceptInvitation: (token: string, password?: string) =>
    request<SessionResponse>("POST", `/auth/invitations/${token}/accept`, {
      password: password || undefined,
    }),

  platform: {
    listClubs: (page = 1) =>
      request<Page<ClubDto>>("GET", `/platform/clubs?page=${page}`),
    createClub: (name: string, slug: string) =>
      request<ClubDto>("POST", "/platform/clubs", { name, slug }),
    updateClub: (clubId: string, patch: { name?: string; status?: string }) =>
      request<ClubDto>("PUT", `/platform/clubs/${clubId}`, patch),
    inviteClubAdmin: (
      clubId: string,
      invitee: { email: string; first_name: string; last_name: string },
    ) =>
      request<unknown>("POST", `/platform/clubs/${clubId}/invitations`, invitee),
    listGlobalAdmins: () =>
      request<UserDto[]>("GET", "/platform/global-admins"),
    addGlobalAdmin: (invitee: {
      email: string;
      first_name: string;
      last_name: string;
    }) => request<unknown>("POST", "/platform/global-admins", invitee),
    removeGlobalAdmin: (userId: string) =>
      request<unknown>("DELETE", `/platform/global-admins/${userId}`),
    setUserStatus: (userId: string, status: "active" | "disabled") =>
      request<unknown>("PUT", `/platform/users/${userId}/status`, { status }),
  },

  club: (slug: string) => ({
    context: () => request<ClubContext>("GET", `/clubs/${slug}`),
    addMember: (email: string, roleId: string) =>
      request<MembershipDto>("POST", `/clubs/${slug}/memberships`, {
        email,
        role_id: roleId,
      }),
    updateMembership: (
      membershipId: string,
      patch: { role_id?: string; status?: string },
    ) =>
      request<unknown>("PUT", `/clubs/${slug}/memberships/${membershipId}`, patch),
    invitations: (page = 1) =>
      request<Page<InvitationDto>>(
        "GET",
        `/clubs/${slug}/invitations?page=${page}`,
      ),
    createInvitation: (invitee: {
      email: string;
      first_name: string;
      last_name: string;
      role_id: string;
    }) => request<InvitationDto>("POST", `/clubs/${slug}/invitations`, invitee),
    resendInvitation: (invitationId: string) =>
      request<InvitationDto>(
        "POST",
        `/clubs/${slug}/invitations/${invitationId}/resend`,
      ),
    revokeInvitation: (invitationId: string) =>
      request<unknown>("DELETE", `/clubs/${slug}/invitations/${invitationId}`),
    roles: () => request<RoleDto[]>("GET", `/clubs/${slug}/roles`),
    setRolePermissions: (roleId: string, permissions: string[]) =>
      request<unknown>("PUT", `/clubs/${slug}/roles/${roleId}/permissions`, {
        permissions,
      }),
    modules: () => request<ModuleDto[]>("GET", `/clubs/${slug}/modules`),
    setModule: (module: string, enabled: boolean) =>
      request<unknown>("PUT", `/clubs/${slug}/modules`, { module, enabled }),
    audit: (page = 1) =>
      request<Page<AuditEntryDto>>(
        "GET",
        `/clubs/${slug}/audit-log?page=${page}`,
      ),
    impersonate: (userId: string) =>
      request<SessionResponse>("POST", `/clubs/${slug}/impersonate`, {
        user_id: userId,
      }),

    // Users module: members list with diving badges + member profiles.
    membersWithBadges: (page = 1) =>
      request<Page<MemberListItem>>("GET", `/clubs/${slug}/members?page=${page}`),
    memberProfile: (userId: string) =>
      request<ProfileResponse>("GET", `/clubs/${slug}/members/${userId}/profile`),
    updateMemberProfile: (userId: string, data: ProfileFields) =>
      request<ProfileResponse>(
        "PUT",
        `/clubs/${slug}/members/${userId}/profile`,
        data,
      ),
    recordCertification: (
      userId: string,
      body: { certification: string; obtained_on?: string | null; validated?: boolean },
    ) =>
      request<ProfileResponse>(
        "POST",
        `/clubs/${slug}/members/${userId}/certifications`,
        body,
      ),
    deleteMemberCertification: (userId: string, certificationId: string) =>
      request<ProfileResponse>(
        "DELETE",
        `/clubs/${slug}/members/${userId}/certifications/${certificationId}`,
      ),
    setTrainingStatus: (userId: string, validUntil: string | null) =>
      request<ProfileResponse>(
        "PUT",
        `/clubs/${slug}/members/${userId}/training-status`,
        { stagiaire_mf1_valid_until: validUntil },
      ),
    memberStamp: (userId: string) =>
      requestBlob(`/clubs/${slug}/members/${userId}/stamp`),
    putMemberStamp: (userId: string, imageBase64: string) =>
      request<unknown>("PUT", `/clubs/${slug}/members/${userId}/stamp`, {
        image_base64: imageBase64,
      }),
    deleteMemberStamp: (userId: string) =>
      request<unknown>("DELETE", `/clubs/${slug}/members/${userId}/stamp`),
  }),
};
