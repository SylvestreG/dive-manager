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
  // auth endpoints also answer 401 (expired link) and must
  // not purge a legitimate session state.
  const isPublicAuthPath =
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
  /** Optional since ADR-0009 (members imported without an address). */
  email: string | null;
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
/** Member intake (POST /clubs/{slug}/members) — ADR-0009. */
export interface NewMember {
  first_name: string;
  last_name: string;
  email?: string;
  birthdate?: string;
  phone?: string;
  licence_number?: string;
  role_id: string;
  send_welcome?: boolean;
  /** Confirms a previous "check" outcome (email known, names differ). */
  confirm_existing?: boolean;
}
/** A stored value that differs from what was entered (never overwritten).
 * Values are null when the caller may not view member data. */
export interface FieldDiff {
  field: "name" | "birthdate" | "phone" | "licence_number";
  stored: string | null;
  incoming: string | null;
}
export interface IntakeUser {
  id: string;
  email: string | null;
  first_name: string;
  last_name: string;
}
export interface AddMemberResult {
  user: IntakeUser;
  /** Null for "check": nothing was written. */
  membership_id: string | null;
  role_id: string;
  role_name: string;
  is_admin: boolean;
  outcome: "created" | "added_existing" | "check";
  diffs: FieldDiff[];
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

// ---- competencies module (global tree, user-global validations) ----

/** The fixed validation ladder (hardcoded backend-side, no stage table). */
export type StageCode = "vu" | "a_perfectionner" | "valide";

export interface LevelRules {
  certification: string;
  label: string;
  /** Above N1: the final validation necessarily happens at sea. */
  sea_validation: boolean;
  /** "E1".."E4" — fixed function of the level. */
  required_validator_grade: string;
  stages: StageCode[];
}
export interface LevelSummary extends LevelRules {
  domains_count: number;
  has_document: boolean;
  page_count: number | null;
}
export interface CompetencyProgress {
  total: number;
  validated: number;
  in_progress: number;
  not_started: number;
  percentage: number;
}
export interface UserRef {
  id: string;
  first_name: string;
  last_name: string;
}
export interface SkillValidationView {
  stage: StageCode;
  validated_at: string;
  validated_by: UserRef | null;
  notes: string | null;
}
export interface SkillView {
  id: string;
  name: string;
  description: string | null;
  validation: SkillValidationView | null;
}
export interface ModuleView {
  id: string;
  name: string;
  progress: CompetencyProgress;
  skills: SkillView[];
}
export interface DomainView {
  id: string;
  name: string;
  progress: CompetencyProgress;
  modules: ModuleView[];
}
export interface CompetencyView {
  level: LevelRules;
  progress: CompetencyProgress;
  domains: DomainView[];
}
export interface TreeSkill {
  id: string;
  name: string;
  description: string | null;
  sort_order: number;
}
export interface TreeModule {
  id: string;
  name: string;
  sort_order: number;
  skills: TreeSkill[];
}
export interface TreeDomain {
  id: string;
  name: string;
  sort_order: number;
  modules: TreeModule[];
}
export interface TreeResponse {
  level: LevelRules;
  domains: TreeDomain[];
}
export interface CompetencyStudent {
  user: UserDto;
  progress: CompetencyProgress;
}
export interface StudentsResponse extends Page<CompetencyStudent> {
  level: LevelRules;
}
export interface LevelDocumentInfo {
  certification: string;
  file_name: string;
  page_count: number;
}
export interface StampPosition {
  skill_id: string;
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
  font_size: number;
  /** Template re-uploaded with fewer pages: shown by the editor, skipped at
   * generation. */
  out_of_range: boolean;
}
export interface CascadeImpact {
  validations_count: number;
}

// ---------- endpoints ----------

export const api = {
  requestMagicLink: (email: string) =>
    request<{ message: string }>("POST", "/auth/magic-link", { email }),
  consumeMagicLink: (token: string) =>
    request<SessionResponse>("POST", "/auth/magic-link/consume", { token }),
  logout: () => request<unknown>("POST", "/auth/logout"),
  logoutAll: () => request<unknown>("POST", "/auth/logout-all"),
  stopImpersonation: () =>
    request<SessionResponse>("POST", "/auth/stop-impersonation"),
  me: () => request<MeResponse>("GET", "/me"),
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

  // Global competency referential (platform-global tree; validations follow
  // the user across clubs — product decision 2026-08-19).
  competencies: {
    levels: () => request<LevelSummary[]>("GET", "/competencies/levels"),
    tree: (certification: string) =>
      request<TreeResponse>(
        "GET",
        `/competencies/tree?certification=${encodeURIComponent(certification)}`,
      ),
    my: (certification?: string) =>
      request<CompetencyView>(
        "GET",
        certification
          ? `/me/competencies?certification=${encodeURIComponent(certification)}`
          : "/me/competencies",
      ),
    myDocument: (certification?: string) =>
      requestBlob(
        certification
          ? `/me/competencies/document?certification=${encodeURIComponent(certification)}`
          : "/me/competencies/document",
      ),

    // Referential management — real global admins only.
    createDomain: (body: { certification: string; name: string }) =>
      request<TreeDomain>("POST", "/competencies/domains", body),
    updateDomain: (
      domainId: string,
      body: { certification: string; name: string; sort_order: number },
    ) => request<TreeDomain>("PUT", `/competencies/domains/${domainId}`, body),
    domainImpact: (domainId: string) =>
      request<CascadeImpact>("GET", `/competencies/domains/${domainId}/impact`),
    deleteDomain: (domainId: string) =>
      request<unknown>("DELETE", `/competencies/domains/${domainId}`),
    createModule: (body: { domain_id: string; name: string }) =>
      request<TreeModule>("POST", "/competencies/modules", body),
    updateModule: (
      moduleId: string,
      body: { name: string; sort_order: number },
    ) => request<TreeModule>("PUT", `/competencies/modules/${moduleId}`, body),
    moduleImpact: (moduleId: string) =>
      request<CascadeImpact>("GET", `/competencies/modules/${moduleId}/impact`),
    deleteModule: (moduleId: string) =>
      request<unknown>("DELETE", `/competencies/modules/${moduleId}`),
    createSkill: (body: {
      module_id: string;
      name: string;
      description?: string | null;
    }) => request<TreeSkill>("POST", "/competencies/skills", body),
    updateSkill: (
      skillId: string,
      body: { name: string; description: string | null; sort_order: number },
    ) => request<TreeSkill>("PUT", `/competencies/skills/${skillId}`, body),
    skillImpact: (skillId: string) =>
      request<CascadeImpact>("GET", `/competencies/skills/${skillId}/impact`),
    deleteSkill: (skillId: string) =>
      request<unknown>("DELETE", `/competencies/skills/${skillId}`),

    // Level documents + stamp positions — real global admins only.
    uploadDocument: (
      certification: string,
      body: { file_name: string; pdf_base64: string },
    ) =>
      request<LevelDocumentInfo>(
        "PUT",
        `/competencies/levels/${encodeURIComponent(certification)}/document`,
        body,
      ),
    downloadDocument: (certification: string) =>
      requestBlob(
        `/competencies/levels/${encodeURIComponent(certification)}/document`,
      ),
    deleteDocument: (certification: string) =>
      request<unknown>(
        "DELETE",
        `/competencies/levels/${encodeURIComponent(certification)}/document`,
      ),
    positions: (certification: string) =>
      request<StampPosition[]>(
        "GET",
        `/competencies/levels/${encodeURIComponent(certification)}/positions`,
      ),
    savePosition: (
      certification: string,
      skillId: string,
      body: {
        page: number;
        x: number;
        y: number;
        width: number;
        height: number;
        font_size?: number;
      },
    ) =>
      request<unknown>(
        "PUT",
        `/competencies/levels/${encodeURIComponent(certification)}/positions/${skillId}`,
        body,
      ),
    deletePosition: (certification: string, skillId: string) =>
      request<unknown>(
        "DELETE",
        `/competencies/levels/${encodeURIComponent(certification)}/positions/${skillId}`,
      ),
  },

  platform: {
    listClubs: (page = 1) =>
      request<Page<ClubDto>>("GET", `/platform/clubs?page=${page}`),
    createClub: (name: string, slug: string) =>
      request<ClubDto>("POST", "/platform/clubs", { name, slug }),
    updateClub: (clubId: string, patch: { name?: string; status?: string }) =>
      request<ClubDto>("PUT", `/platform/clubs/${clubId}`, patch),
    addClubAdmin: (
      clubId: string,
      person: { email: string; first_name: string; last_name: string },
    ) =>
      request<AddMemberResult>("POST", `/platform/clubs/${clubId}/admins`, person),
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
    // ADR-0009: creates the account, or adds the one matching the email.
    addMember: (member: NewMember) =>
      request<AddMemberResult>("POST", `/clubs/${slug}/members`, member),
    updateMembership: (
      membershipId: string,
      patch: { role_id?: string; status?: string },
    ) =>
      request<unknown>("PUT", `/clubs/${slug}/memberships/${membershipId}`, patch),
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
    // Competencies module: students preparing a level, member competency
    // sheet, skill validations, filled level document.
    competencyStudents: (certification: string, page = 1) =>
      request<StudentsResponse>(
        "GET",
        `/clubs/${slug}/competencies/students?certification=${encodeURIComponent(certification)}&page=${page}`,
      ),
    memberCompetencies: (userId: string, certification: string) =>
      request<CompetencyView>(
        "GET",
        `/clubs/${slug}/members/${userId}/competencies?certification=${encodeURIComponent(certification)}`,
      ),
    memberCompetenciesDocument: (userId: string, certification: string) =>
      requestBlob(
        `/clubs/${slug}/members/${userId}/competencies/document?certification=${encodeURIComponent(certification)}`,
      ),
    putSkillValidation: (
      userId: string,
      skillId: string,
      body: {
        stage: StageCode;
        validated_at?: string;
        notes?: string;
        /** Real global admins only: attribute the validation to a named
         * instructor ("valider au nom d'un encadrant"). */
        validated_by_user_id?: string;
      },
    ) =>
      request<CompetencyView>(
        "PUT",
        `/clubs/${slug}/members/${userId}/skill-validations/${skillId}`,
        body,
      ),
    deleteSkillValidation: (userId: string, skillId: string) =>
      request<CompetencyView>(
        "DELETE",
        `/clubs/${slug}/members/${userId}/skill-validations/${skillId}`,
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
