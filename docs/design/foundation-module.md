# Foundation module design — `identity` + `tenancy`

Status: **implemented** (2026-08-17, `backend/`). Implements ADR-0002,
ADR-0003, ADR-0004, ADR-0006.

**Deviations decided during implementation:**
1. `citext` replaced by lowercase `TEXT` + `CHECK (email = lower(email))` —
   same guarantee, expressible in pure SeaORM DSL (ADR-0006), no extension.
2. **Impersonation club scoping** (tenant-isolation fix found in review): a
   club admin's impersonation session carries `impersonated_club_id` and is
   rejected outside that club — otherwise impersonating a member who belongs
   to several clubs would leak access into the other clubs. Global-admin
   impersonation is unscoped. `/me` under a scoped session only reveals the
   scoped club's membership.
3. The "router-walk audit test" is implemented as per-endpoint negative tests
   instead (axum has no route introspection): every club endpoint is exercised
   with a permissionless member, a foreign club admin, and cross-club resource
   ids (see `src/tenancy/tests/`).
4. A magic-link email send failure is logged, never surfaced — a 500 would be
   an account-existence oracle on an otherwise generic endpoint.
5. **OIDC Google is the immediate next slice** (schema ready:
   `user_oidc_identities`); magic links + passwords + invitations cover all
   logins meanwhile.

**Senior-review fixes applied (2026-08-17, H1–H4/M1–M8/L2–L9):** unique
violations mapped to 409 (`DbErr::Exec` was unhandled); module boundaries
restored (all `users`-table writes via `identity::services::users`; `/me`
composition moved to tenancy); Argon2 on the blocking pool; composite FKs
`(club_id, role_id)` on memberships/invitations; last-admin guard serialized
with row locks and audits moved inside mutation transactions; `trust_proxy`
config gating `X-Forwarded-For`; test-capture mailer compiled out of release;
platform user enable/disable endpoint (sessions revoked on disable, last
global-admin guard); paginated `{items,total,page,per_page}` envelopes;
expiry/cleanup indexes + partial unique pending-invitation index (justified
raw DDL); config-file misconfiguration surfaced; successful logins no longer
consume the rate budget; `/healthz`; quieter sqlx logging.

## 1. Scope

- `identity`: users, credentials (OIDC identities, passwords, login magic
  links), sessions, invitations, bootstrap, audit log.
- `tenancy`: clubs, memberships, club roles & permissions, module toggles,
  tenant context extraction, impersonation.

Out of scope here: member profiles (module `members` — but the profile ↔ user
activation link is specified because it constrains `identity`), questionnaire
magic links (module `registrations`), club SMTP (module `notifications`; the
foundation uses one **platform-level** SMTP config to send invitations and
login links).

## 1bis. Fosse source analysis — facts vs assumptions

**Facts (read in Fosse's code — details in `docs/analysis/fosse-backend.md`):**
auth = Google OAuth code flow + One Tap + email/password + 6-char temp
passwords (24 h); JWT HS256 24 h carrying `is_admin` derived solely from the
config `admin.emails` list; Google login allowed only for config-admins or
existing `people` rows (case-sensitive email match); ACL = 3 seeded groups ×
22 permissions, matrix editable via `PUT /groups/:id`; **fail-open** when the
person is unknown or ungrouped; most mutation routes authenticated but not
authorized; impersonation admin-only with "real admin" gates; no rate
limiting, no revocation, `must_change_password` advisory; DB seed omits
`competencies_validate` from every group.

**Assumptions (inferred, unverified):** the temp-password flow is the main
member onboarding path; the fail-open branch may be **load-bearing** in
production (because of the seed gap, encadrants can only validate if the
matrix was edited *or* they are ungrouped); the permission matrix was possibly
never customized.

**Resolved by the product owner (2026-08-17):** Fosse's groups/permission
matrix is **not migrated** — Dive Manager recreates all roles from its seeded
defaults (the importer maps people to seeded roles only). Impersonation for
**both** global and club admins, single-level with switch semantics (§5).
OIDC at launch: **Google only** (Microsoft later). Session lifetimes
confirmed: 30 d sliding / 90 d max. Strong project rule: **SeaORM for
migrations and queries, no raw SQL** (ADR-0006).

**Still needed at deploy time:** the bootstrap global-admin email(s) — a
config value, not a design blocker.

## 2. Schema (PostgreSQL)

Conventions: UUIDv7 PKs (time-ordered, index-friendly), `created_at`/
`updated_at` `timestamptz` set by the application, FKs `ON DELETE RESTRICT`
unless stated, all secrets stored **hashed**.

### identity

```
users
  id            uuid PK
  email         citext UNIQUE NOT NULL        -- citext kills Fosse's case bugs
  first_name    text NOT NULL
  last_name     text NOT NULL
  status        text NOT NULL CHECK (status IN ('invited','active','disabled'))
  is_global_admin boolean NOT NULL DEFAULT false
  created_at, updated_at

user_passwords                                 -- optional, 0..1 per user
  user_id       uuid PK → users ON DELETE CASCADE
  password_hash text NOT NULL                  -- Argon2id
  updated_at

user_oidc_identities
  id            uuid PK
  user_id       uuid → users ON DELETE CASCADE
  provider      text NOT NULL                  -- 'google' | 'microsoft' | …
  subject       text NOT NULL                  -- OIDC `sub`
  email_at_link timestamptz-stamped snapshot of the provider email
  created_at
  UNIQUE (provider, subject)
  UNIQUE (user_id, provider)                   -- one identity per provider per user

login_magic_links
  id            uuid PK
  user_id       uuid → users ON DELETE CASCADE
  token_hash    bytea UNIQUE NOT NULL          -- SHA-256 of the emailed token
  expires_at    timestamptz NOT NULL           -- 12 h (product decision, config value)
  consumed_at   timestamptz NULL
  invalidated_at timestamptz NULL              -- superseded by a newer link or a login
  created_at
  INDEX (user_id, created_at)                  -- rate-limit window queries

auth_sessions
  id            uuid PK
  user_id       uuid → users ON DELETE CASCADE
  token_hash    bytea UNIQUE NOT NULL          -- opaque bearer token, hashed
  impersonated_user_id uuid NULL → users       -- set while impersonating
  created_at, last_seen_at
  expires_at    timestamptz NOT NULL           -- sliding, e.g. 30 days idle cap
  revoked_at    timestamptz NULL
  INDEX (user_id)

invitations
  id            uuid PK
  email         citext NOT NULL
  club_id       uuid NULL → clubs ON DELETE CASCADE   -- NULL = global-admin invite
  role_id       uuid NULL → club_roles                -- role granted on acceptance
  invited_by    uuid NOT NULL → users
  token_hash    bytea UNIQUE NOT NULL
  expires_at    timestamptz NOT NULL           -- 7 days, resendable
  consumed_at   timestamptz NULL
  revoked_at    timestamptz NULL
  created_at
  INDEX (email), INDEX (club_id)

audit_log
  id            uuid PK
  actor_user_id uuid NOT NULL → users          -- the REAL user
  impersonated_user_id uuid NULL → users
  club_id       uuid NULL → clubs
  action        text NOT NULL                  -- 'invitation.created', 'role.granted', …
  entity_type   text NOT NULL, entity_id uuid NULL
  details       jsonb NOT NULL DEFAULT '{}'
  created_at
  INDEX (club_id, created_at), INDEX (actor_user_id, created_at)
```

### tenancy

```
clubs
  id            uuid PK
  name          text NOT NULL
  slug          citext UNIQUE NOT NULL         -- URL/identifier, e.g. 'usi'
  status        text NOT NULL CHECK (status IN ('active','suspended'))
  created_at, updated_at

club_roles                                     -- per-club, seeded on club creation
  id            uuid PK
  club_id       uuid → clubs ON DELETE CASCADE
  name          text NOT NULL                  -- 'Administrateur', 'Encadrant', 'Membre'
  is_admin      boolean NOT NULL DEFAULT false -- shortcut: full club permissions
  UNIQUE (club_id, name)

club_role_permissions
  role_id       uuid → club_roles ON DELETE CASCADE
  permission    text NOT NULL                  -- from the code-side catalogue
  PRIMARY KEY (role_id, permission)

memberships
  id            uuid PK
  club_id       uuid → clubs ON DELETE CASCADE
  user_id       uuid → users ON DELETE CASCADE
  role_id       uuid NOT NULL → club_roles
  status        text NOT NULL CHECK (status IN ('active','suspended'))
  created_at, updated_at
  UNIQUE (club_id, user_id)                    -- one membership (one role) per club
  INDEX (user_id)

club_modules
  club_id       uuid → clubs ON DELETE CASCADE
  module        text NOT NULL                  -- 'competencies', 'imports', …
  enabled       boolean NOT NULL
  PRIMARY KEY (club_id, module)
```

Notes:
- **One role per membership** (Fosse: one group per person). Simpler; role
  permissions are editable per club, which covers the "encadrant with extra
  rights" case without multi-role complexity. Revisit only with a real case.
- Permission **catalogue lives in code** (one Rust enum, like Fosse's 22) —
  each business module contributes its permissions. Grants live in DB. Unknown
  DB strings are ignored on read (forward compat), never granted.
- Member profiles (module `members`) will reference `club_id` + optional
  `user_id`; a profile with `user_id NULL` is a not-yet-activated member
  (ADR-0004 §2). Activation = invitation consumed → user created/linked →
  membership created with the Membre role → profile.user_id set.

## 3. Authorization model

### Principals and roles

- **Global admin** (`users.is_global_admin`): platform management, club CRUD,
  can act in any club (every such cross-tenant action is audited). Not a club
  role — no membership required.
- **Club role** via membership: seeded roles per club — Administrateur
  (`is_admin`, all club permissions), Encadrant, Membre — permission sets
  editable by the club admin (Fosse's GroupsPage UX, kept, but fail-closed
  and per club).

### Foundation permission catalogue (v1)

```
club.manage            -- club settings, module toggles
club.members.view / club.members.manage
club.roles.manage      -- edit role permission sets, assign roles
club.invitations.manage
club.impersonate       -- impersonate members of this club
```

Business modules add their own (`sessions.manage`, `competencies.validate`, …)
in their own designs. **Default seeded sets**: Administrateur = all;
Encadrant = `club.members.view`; Membre = none of the above (members reach
their own data through ownership rules, not permissions).

### Enforcement

- Every protected route declares its requirement in one place
  (`require(permission)` / `require_global_admin()` / ownership check).
  An automated test walks the router and fails on any mutating route without a
  declared requirement — the direct answer to Fosse's biggest hole.
- Deny by default: no membership ⇒ no club access; unknown permission ⇒ deny;
  suspended membership/club ⇒ deny.

## 4. Tenant context

- API shape: club-scoped resources live under `/api/v1/clubs/{club_slug}/…`.
  The tenant is explicit in the URL, **verified** against the caller's
  membership (or global-admin status) by middleware — never trusted from a
  body field.
- Middleware output: `TenantContext { club_id, membership_role, permissions,
  acting_user, real_user }`. Handlers receive it typed; club-owned repository
  methods **require** it as a parameter and always filter `club_id = ctx.club_id`.
  There is no repository API for club-owned tables without a context.
- Global admin acting inside a club gets a context flagged `via_global_admin`
  (audited).
- Non-club routes: `/api/v1/me/*` (my clubs, my sessions), `/api/v1/platform/*`
  (global admin), `/api/v1/auth/*` (public).

## 5. Authentication flows

### Bootstrap
At startup: if `users WHERE is_global_admin` is empty, create `invited` global
admins from `config.bootstrap_admin_emails` and log it. Otherwise the setting
is ignored entirely. First login by such a user goes through the normal
activation path (magic link to their email).

### Login — magic link (default)
1. `POST /auth/magic-link {email}` → always the same generic 200 (no user
   enumeration). If the user exists and is active/invited: create token (32
   random bytes, base64url; store SHA-256), email `{base}/auth/ml/{token}`,
   valid **12 h** (product decision — long enough for evening email readers;
   config value).
2. Mitigations for the long validity: **strictly single-use** (`consumed_at`);
   **requesting a new link invalidates all previous ones** (at most one valid
   link per account at any time); **any successful login by any method
   invalidates the account's pending links**.
3. `POST /auth/magic-link/consume {token}` → creates an `auth_session`,
   returns the opaque session token. Activates an `invited` user (with an
   unconsumed matching invitation) on first use.
4. Rate limits: 3 requests / email / 15 min; per-IP budget. In-process limiter
   (single instance — no Redis).
5. Usage guidance (not enforced): club/global admins should prefer OIDC or
   password; their mailbox is a higher-value target.

### Login — password (optional)
`POST /auth/login {email, password}` — Argon2id verify, constant-time-ish
behavior on unknown email (dummy verify), same rate limiting + progressive
per-account lockout. Setting a password happens from the logged-in profile
("add a password") — there is no separate temp-password path (Fosse's
brute-forceable 6-char temp password is dropped; "forgot password" = magic
link, which already covers it).

### Login — OIDC (Google, Microsoft; platform-level config)
1. Authorization-code flow **with PKCE and nonce**; providers declared in
   platform config (issuer URL, client id/secret); discovery + **local JWKS
   validation** of the id_token (`iss`, `aud`, `exp`, `nonce`,
   `email_verified == true`). No `tokeninfo` round-trip (Fosse's weakness).
2. Identity resolution, in order:
   - `(provider, subject)` matches → log that user in.
   - Else if provider email (verified) matches an existing user or a pending
     invitation → link identity to that account (activating it if `invited`),
     log in, audit `oidc.linked`.
   - Else → reject with "invitation required" (invitation-only platform).
3. Never auto-create accounts from OIDC alone; never match by email for an
   already-linked provider identity.

### Sessions
Opaque 32-byte bearer token, stored hashed; sliding `expires_at` (idle
timeout ~30 days, absolute cap ~90); `revoked_at` for logout / "log out
everywhere" / admin revocation. Role and admin changes are effective on the
next request because authorization always reads current DB state — nothing
authorization-relevant is baked into the token.

### Invitations
- Global admin → invite global admin (club_id NULL) or create club + invite
  its Administrateur. Club admin (with `club.invitations.manage`) → invite
  members/encadrants; bulk path = CSV import (module `imports`) which creates
  profiles + invitations.
- Acceptance: `GET/POST /auth/invitations/{token}` → create-or-find user by
  email (citext), create membership with the invitation's role, link member
  profile if one is waiting on that email, mark consumed, open a session.
  Choosing a password at activation is offered but optional (ADR-0004).
- Invitations are resendable (new token, old one revoked), revocable, expire
  after 7 days.

### Impersonation
- Allowed to **global admins (any user)** and **club admins (members of their
  club only)** — product owner, 2026-08-17.
- `POST /clubs/{slug}/impersonate {user_id}`: requires `club.impersonate` (or
  global admin); target must be a member of that club; forbidden if the target
  is a global admin. Sets `auth_sessions.impersonated_user_id` on a **new**
  session; `real_user` keeps their own session untouched.
- **No chained impersonation** — the design uses *switch* semantics instead:
  impersonation is always a single level `real_user → target`. If a global
  admin impersonating a club admin clicks "impersonate member X", the target
  simply **switches** to X. The permission check is always evaluated against
  the **real** user (so an impersonated club admin can never be used by a
  non-authorized real user to reach someone, and a real club admin can only
  switch within their club). Same UX benefit as chaining, none of the audit
  and privilege-reasoning complexity; every switch is audited with
  `real → old target → new target`.
- While impersonating: permissions are the target's; destructive/authz-
  sensitive actions additionally require a non-impersonating principal (same
  "real admin" semantics as Fosse, kept). Start/stop/switch audited.

## 5bis. API surface (v1)

Public (`/api/v1/auth/*`):
```
POST /auth/magic-link                 {email}            → generic 200
POST /auth/magic-link/consume         {token}            → session token + me
POST /auth/login                      {email, password}  → session token + me
GET  /auth/oidc/{provider}/start                         → redirect (PKCE+nonce+state)
GET  /auth/oidc/{provider}/callback   ?code&state        → session token + me
GET  /auth/invitations/{token}                           → invitation preview
POST /auth/invitations/{token}/accept {password?}        → session token + me
```

Authenticated (`/api/v1/me`, `/api/v1/auth`):
```
GET  /me                       → identity, global-admin flag, memberships+permissions
POST /auth/logout              → revoke current session
POST /auth/logout-all          → revoke all my sessions
PUT  /me/password              → set/change password (optional credential)
POST /auth/stop-impersonation  → revoke impersonated session, return own
```

Platform (global admin, `/api/v1/platform/*`):
```
GET/POST        /platform/clubs                    → list / create (seeds roles+modules)
PUT             /platform/clubs/{id}               → rename / suspend
GET/POST/DELETE /platform/global-admins            → manage global admins (audited)
POST            /platform/clubs/{id}/invitations   → invite the club's Administrateur
```

Club-scoped (`/api/v1/clubs/{slug}/*`, membership-checked):
```
GET/PUT   /clubs/{slug}                      → club settings           [club.manage]
GET/PUT   /clubs/{slug}/modules              → module toggles          [club.manage]
GET       /clubs/{slug}/roles                → roles + permission sets [club.roles.manage]
PUT       /clubs/{slug}/roles/{id}/permissions                         [club.roles.manage]
GET       /clubs/{slug}/memberships          → members + roles         [club.members.view]
PUT       /clubs/{slug}/memberships/{id}     → change role / suspend   [club.roles.manage]
GET/POST  /clubs/{slug}/invitations          → list / invite           [club.invitations.manage]
POST      /clubs/{slug}/invitations/{id}/resend                        [club.invitations.manage]
DELETE    /clubs/{slug}/invitations/{id}     → revoke                  [club.invitations.manage]
POST      /clubs/{slug}/impersonate {user_id}                          [club.impersonate]
GET       /clubs/{slug}/audit-log            → paginated               [club.manage]
```
All list endpoints paginated (`?page`/`per_page`, capped).

## 5ter. Domain model (Rust shape, indicative)

- Crate/module `identity`: `User`, `AuthSession`, `Invitation`,
  `Credential` operations (password, oidc identity, magic link);
  services `AuthService` (login flows, token issuing/verification),
  `InvitationService`, `AuditService`. No club knowledge.
- Crate/module `tenancy`: `Club`, `Membership`, `Role`,
  `Permission` (enum, code-side catalogue), `TenantContext`;
  `AuthzService::require(ctx, permission)`; Axum extractors:
  `CurrentUser` (session → user) and `Tenant` (slug + membership →
  `TenantContext`), so handlers declare their needs in their signature.
- Repositories per aggregate; club-owned repositories take `&TenantContext`.

## 5quater. Performance

- Auth cost per request: 1 indexed point query (`auth_sessions.token_hash`)
  + 1 join query (membership → role → permissions) on club routes. At club
  scale this is negligible; an in-process TTL cache can be added later without
  API change.
- `last_seen_at` updated at most once per minute per session (avoid a write
  per request).
- Expired sessions/magic links/invitations purged by a periodic in-process
  task (tokio interval) — no external job infrastructure.
- In-memory rate limiter (single instance); audit writes synchronous (low
  volume: auth/admin events only).

## 6. Config (platform)

```
server   { host, port }
database { url }                       # env override: DATABASE_URL
auth     { bootstrap_admin_emails: [], session_idle_days, session_max_days }
oidc     [ { name, issuer, client_id, client_secret } ]   # secrets via env
smtp     { host, port, username, password, from_email, from_name }  # optional:
                                       # absent ⇒ invitations/links logged, not sent (dev)
base_url                               # link generation
```

File + env-var overrides; secrets preferably env-only. Missing SMTP degrades
(dev mode), never blocks boot (Fosse hard-fails on a missing smtp block).

## 7. Security checklist (design-time answers)

- IDOR: tenant in URL + membership check + repository-level club_id filter.
- Privilege escalation: fail-closed permissions; role edits require
  `club.roles.manage`; nobody can grant global admin except a global admin;
  impersonation can't reach global admins.
- Token theft: all tokens hashed at rest; magic links single-use + 15 min;
  sessions revocable; no secrets in URLs except single-use tokens delivered
  by email.
- Enumeration: generic responses on magic-link/password endpoints.
- Brute force: per-account + per-IP rate limits, progressive lockout.
- CSRF: bearer-token API (no ambient cookie auth in v1); revisit if the
  frontend choice introduces cookies.
- Audit: invitations, role changes, global-admin changes, impersonation,
  cross-tenant global-admin actions, OIDC linking.

## 8. Tests required (Definition of Done)

- Route audit: every mutating route declares a permission/ownership rule.
- Tenant isolation: member of club A cannot read/mutate club B via every
  club-scoped endpoint (parameterized test over the router).
- Fail-closed: user without membership, suspended membership, suspended club,
  unknown permission string, empty role ⇒ deny.
- Auth: magic-link single-use/expiry/rate-limit, newer-link and
  successful-login invalidation; OIDC linking matrix (known
  subject / known email / unknown / unverified email / second provider);
  password lockout; session revocation immediacy; invitation
  consume/expire/revoke/resend; impersonation permission boundaries.
- Bootstrap idempotence (existing global admin ⇒ config ignored).

## 9. Open items

- Rust HTTP framework + DB layer decision (ARCHITECTURE.md open table) — to
  settle when the workspace skeleton is created, right after this design is
  approved.
- Exact session lifetimes and lockout thresholds — proposed values above,
  tune at review.
- `club_modules` enforcement point (middleware 404 vs per-module check) —
  proposal: middleware rejects club-scoped routes of disabled modules.
