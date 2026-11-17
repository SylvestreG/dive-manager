# Auth & member onboarding redesign — `identity` rework + `onboarding` module

Status: **approved** (product owner, 2026-11-16) — ADR-0009 accepted.
Slice 1 implemented (2026-11-16). Slice 2 implemented (2026-11-17) together
with Sign in with Apple **for the web** (the iOS native flow stays in slice 5);
slices 3–6 pending.
Supersedes the authentication parts of ADR-0004 and the foundation design
(`foundation-module.md` §5 login/invitations). Proposed as **ADR-0009**
(text in §14). Nothing here touches Fosse: Fosse keeps its own auth
(Google/passwords/JWT) until decommission.

## 0. Product decisions this design implements (2026-11-16)

1. SSO **Google + Microsoft** first; Facebook later on the same model.
2. **Passwords removed** (passkeys in a later slice).
3. **iOS app short-term** ⇒ Sign in with Apple + native SSO flow are in scope
   (slice 5).
4. **Club admin power**: may link a login identity only to a member of *their*
   club who **never logged in** (and belongs to no other club); everything else
   is a platform (super) admin decision.
5. **No invitations.** An account exists only because an admin wanted it: CSV
   import, manual creation, or approval of an access request.
6. Login matching order: known identity → trusted matching email (auto-link)
   → untrusted matching email (confirmation link) → access request.
7. Users manage their own login methods; admins resolve the rest.
8. **CSV import with column mapping** (suggested from headers, remembered per
   club, date-format choice), per-row preview, single-transaction commit.
9. Sessions: 180 d idle / 365 d cap (done, PR #3) + "my devices" page.

## 1. Module identification

| | |
|---|---|
| Modules | `identity` (reworked) + **new `onboarding`** |
| `identity` owns | users, login identities (OIDC), magic links, OIDC flows, sessions, audit log, bootstrap |
| `onboarding` owns | club join codes, access requests + review queues, member CSV import (mappings, runs), manual member creation, welcome emails |
| Users involved | members, club admins, platform admins, unauthenticated visitors |
| Depends on | `identity`, `tenancy` (clubs, memberships, roles, permissions), `users` (profile fields: birthdate, phone, licence number) |
| Future dependents | `registrations`/`imports` (session CSV import may reuse the CSV inspection layer), `notifications` (email sending), passkeys |

**IN SCOPE**: removal of passwords and invitations; login identities and
the OIDC flow (Google, Microsoft, Apple); trust and matching rules; the
link-confirmation magic link; magic-link login for users who have an email;
access requests through club join codes, with club and platform review
queues; login-method management in the profile; "my devices" page;
member CSV import with mapping, preview and commit; manual member creation;
welcome emails; data migration from the current tables.

**OUT OF SCOPE**: Facebook (the model is ready for it); passkeys (slice 6
gets its own short design); importing registrations into a session (the
`imports` module); bulk CACI/RIFAP dates (the importer backlog); family
accounts (see open question Q1); club-level SMTP; the Fosse importer.

## 2. Fosse analysis (read-only; Fosse is not modified)

**CONFIRMED** (code-read, details in the agent report of 2026-11-16):
- Google login matches `people.email` **exactly, case-sensitively**. The
  `sub` is never stored and `email_verified` is never checked. Verification
  goes through Google's `tokeninfo` endpoint. An unknown email gets 403
  unless it is in the config admin list.
- Passwords: a 6-char temp password (24 h, emailed), then an optional real
  password. JWT HS256 with `is_admin` taken from the config email list
  (case-sensitive); no revocation.
- "Gmail ⇒ Google only" is **frontend-only**; the backend does not enforce it.
- Fosse magic links are **questionnaire tokens**, not logins.
- One `people` table: email NOT NULL + unique (case-sensitive); **no
  birthdate, no licence number**.
- CSV import: comma-separated, UTF-8 only, fixed columns
  `first_name,last_name,email,phone`. It is always tied to a session (it
  creates questionnaires plus email jobs). It matches by exact email and
  never updates an existing person. No transaction; errors are stored as JSON.
- Email normalization is inconsistent: some paths lowercase and trim, others
  do not. Case-variant duplicates are possible.

**INFERRED**: the temp-password flow is the main onboarding path; Fosse has no
way to attach a second login method other than "same email".

**UNKNOWN**: how many USI members share an email address (family accounts).
Fosse's unique email index makes such duplicates impossible, so they cannot
be observed. This matters for Q1.

**Kept from Fosse**: one-click Google login; the admin decides who gets in.
**Not reproduced**: email-only matching for SSO, case-sensitive emails, temp
passwords, config-file admins, CSV with fixed columns and UTF-8 only.

## 3. Business rules

### 3.1 Accounts
- R1. A **user** (person) is created only by an admin: manual creation, CSV
  import, approval of an access request, or bootstrap (platform admins from
  config when none exists).
- R2. `users.email` becomes **optional** (members imported without an email).
  When present it is unique, case-insensitive (lowercased), and it is the
  address used for magic links.
- R3. `users.activated_at` is set at the **first successful login**, whatever
  the method. Users with `activated_at IS NULL` are "never logged in". The
  `invited` status disappears; `status ∈ {active, disabled}`.
- R4. **Club-managed account** (the central authorization predicate):
  `activated_at IS NULL` AND not a global admin AND the user has memberships in
  **exactly one** club, which is the acting club. Only club-managed accounts
  can be shaped by a club admin's sensitive actions: link a login identity,
  change the email, merge. Everything else needs a platform admin.

### 3.2 Login methods
- R5. Login methods are:
  - a **magic link** to `users.email` (always available when the user has an
    email and is active);
  - **OIDC identities** `(provider, subject)` (Google, Microsoft, Apple;
    Facebook later);
  - passkeys (slice 6).
- R6. An identity is matched **only by `(provider, subject)`** once it is
  linked. An email is used only to find a candidate account for a *first* link.
- R7. **Trusted provider email** (enough for an automatic first link):
  - Google, with `email_verified = true`;
  - Microsoft, only for personal accounts (`tid` =
    `9188040d-6c67-4c5b-b112-36a304b66dad`), because work-tenant emails are
    not ownership-proven;
  - Apple, with `email_verified = true` (private relay addresses simply never
    match).

  Everything else is untrusted.
- R8. **Login resolution** (OIDC callback, in this order):
  1. A known `(provider, subject)` on an active user → session.
     On a disabled user → refused.
  2. A pending access request exists for `(provider, subject)` → "request
     pending" screen (no session).
  3. A trusted email equal to an active user's `users.email` → link the
     identity, set `activated_at` and `email_verified_at` → session. Audited
     as `identity.auto_linked`.
  4. An untrusted email equal to an active user's `users.email` → send a
     **link-confirmation magic link** to that address. Clicking it links the
     identity and opens a session.
  5. Otherwise → **access request form**, pre-filled with the provider's names.
- R9. **Magic-link request** (`POST /auth/magic-link {email}`): the response is
  always the same generic message (no enumeration).
  - Known active user: a login link is emailed.
  - Unknown address: an email is still sent. It says "no account uses this
    address — request access here", with a single-use **registration link**.
    The request it produces therefore carries a *verified* email, and the
    screen never reveals whether the account exists.
- R10. A user may **link** another provider from the profile at any time
  (authenticated + provider proof, no email match needed). Linking fails if
  that `(provider, subject)` already belongs to another user, with the message
  "already used by another account — contact your club".
- R11. A user may **unlink** an identity if at least one login method remains
  afterwards (an email for magic links, or another identity).
- R12. Changing your own `users.email` requires clicking a confirmation link
  sent to the **new** address. An admin may change the email only of a
  club-managed account (R4), or as a platform admin.

### 3.3 Access requests
- R13. An access request targets a **club**, through that club's join code
  (link or QR code), or the **platform** (no club; reviewed by platform admins).
- R14. A request carries:
  - the identity: either `(provider, subject)` with its email and trust flag,
    or a verified email from the registration link;
  - first name, last name, birthdate (required), an optional message, the
    target club.
- R15. At most **one pending request per identity**, platform-wide.
- R16. A requester has **no session and no access** until approval.
- R17. Club review (permission `club.intake.manage`) offers three decisions:
  - **link** to an existing member. Only club-managed accounts (R4); the UI
    lists suggestions (§3.5) first;
  - **create** a new member: a user plus a membership with a non-admin role,
    plus the identity;
  - **reject**, with an optional reason.

  When the right person is not club-managed, the admin **escalates**: the
  request then also appears in the platform queue.
- R18. A platform admin can resolve any request, linking to **any** user, with
  the same decisions. All decisions are audited.
- R19. When the requester has an email address, they are notified of the
  decision by email. Rejected and approved requests are **purged after 90
  days**.

### 3.4 CSV member import
- R20. The importable fields are: last name\*, first name\*, birthdate, email,
  phone, licence number. Every imported member gets **one role, chosen in the
  import screen**. It must be a non-admin role of the club; the default is
  "Membre".
- R21. **Matching** for each row, in order:
  1. email: global, exact after lowercase and trim;
  2. licence number: global; if the result is ambiguous, the row goes to
     "check";
  3. last name + first name + birthdate: global, compared case- and
     accent-insensitively.

  Rows matched by (3) are **"check" rows**: the admin must confirm "same
  person" or choose "new person".
- R22. **Existing users are never overwritten by an import or a manual
  add** (review fix 2026-11-16: profile gaps are filled **only on
  club-managed accounts** (R4); an email match with different names returns
  `check` and writes nothing until the admin confirms; stored profile values
  in the response are hidden from callers without `club.members.view`):
  - empty profile fields are filled;
  - fields that differ are reported in the preview but left unchanged;
  - a matched user who is not yet in the club gets a membership.
- R23. **Rejected rows**: missing last or first name, an unparseable date, an
  invalid email, or an email already used inside the same file. A row whose
  email exists but whose names clearly differ goes to "check".
- R24. The commit **re-validates everything on the server** and runs in **one
  transaction**: all of it or nothing. It records an `import_run` (counts and
  per-row outcome) and is audited.
- R25. The column mapping and date format are **remembered per club**. On the
  next file, the mapping is suggested from the saved one first, then from a
  synonym table (`nom`, `NOM`, `last name`, `Nom de famille` → last name,
  etc.).
- R26. Optional **welcome email** after the commit, sent to the rows that have
  an email. It contains no token: it explains how to log in and links to the
  login page with the email pre-filled.

### 3.5 Suggestions (review queue)
- R27. Candidates for a request are taken among club-managed members in the
  club queue, and among all users in the platform queue. Ranking:
  1. same birthdate **and** similar name: first + last name trigram/normalized
     equality;
  2. same email local-part or same email;
  3. similar name only.

  At most 5 suggestions; the admin can always search the member list.

### 3.6 Sessions & devices
- R28. Each session records a coarse client label, such as "iPhone · app" or
  "Chrome · Mac", derived from the User-Agent. **No IP is stored.**
- R29. The "My devices" page lists my sessions, with "log out this device" and
  "log out everywhere".
- R30. A platform admin can revoke all sessions of any user. Disabling a user
  already revokes them.

## 4. Domain model

`identity`:
- **User**: id, email?, first_name, last_name, status, is_global_admin,
  activated_at?, email_verified_at?.
- **LoginIdentity**: provider (`google | microsoft | apple`, later
  `facebook`), subject, email_at_link?, last_used_at.
- **OidcFlow**: short-lived state of one browser or app round-trip. It holds
  state, nonce, PKCE verifier, purpose (`login | link`), the linking user (for
  `link`), join code, client (`web | ios`), and the verified claims once the
  callback has succeeded.
- **MagicLink**: purpose `login | confirm_link | register | confirm_email`,
  a target user? and its payload.
- **AuthSession**: as today, plus `client_label`.
- **ExchangeCode**: one-time code (60 s) that the web page or the app trades
  for a session token. Session tokens never appear in URLs.

`onboarding`:
- **JoinCode** (per club): code, enabled, rotated_at.
- **AccessRequest**: club?, identity reference (provider+subject, or verified
  email), names, birthdate, message, status `pending | approved | rejected`,
  escalated_at?, decision (who, when, reason, resolved user).
- **ImportMapping** (per club): field → CSV header, date format, last used.
- **ImportRun**: club, actor, file name, counts, per-row outcomes (jsonb,
  bounded).

**State machine — AccessRequest**: `pending → approved | rejected`
(terminal); `escalated_at` is a flag that does not create a separate state.
A `pending` request is withdrawn automatically when the identity gets linked
some other way (for example, the user is created by import and then logs in
through Google): it becomes `approved` with "resolved by login".

**Invariants**:
- `(provider, subject)` is globally unique;
- at most one `pending` request per identity;
- a user has at least one login method if `activated_at` is set;
- an admin role is never assigned by import or by access-request approval
  (admins are promoted afterwards, with the existing `club.roles.manage`
  rules).

## 5. Module boundaries

- `identity` exposes these services:
  - `resolve_login(claims) → LoginOutcome {Session | Pending | ConfirmSent |
    NeedsRegistration(flow)}`;
  - `link_identity(user, identity)`;
  - `create_user(...)` (the only writer of `users`);
  - `is_club_managed(user, club)`, which takes membership facts from
    `tenancy` as input; `identity` stays club-agnostic;
  - `send_magic_link(purpose, ...)`;
  - session operations.
- `onboarding` calls:
  - `identity` for users, identities and links;
  - `tenancy` for memberships and roles;
  - `users` for profile fill-empty (a new `users::services::fill_profile_gaps`).

  It never writes their tables directly.
- `tenancy`: `club.invitations.manage` is **renamed** to `club.intake.manage`;
  invitation code is deleted; `/me` composition is unchanged.
- No cycles: `onboarding → identity, tenancy, users`.

## 6. Multi-tenancy & authorization

| Resource | Owner |
|---|---|
| users, login identities, sessions, magic links, OIDC flows | user / platform (global) |
| user profiles | user (global; filled by club staff, R22) |
| join codes, import mappings, import runs | **club** (`club_id`) |
| access requests | **club** (`club_id`) or platform (`club_id NULL`) |

| Operation | Unauthenticated | Member | Club admin (`club.intake.manage`) | Platform admin |
|---|---|---|---|---|
| OIDC login / magic link | ✅ | ✅ | ✅ | ✅ |
| Submit access request (with join code or platform) | ✅ (rate-limited) | – | – | – |
| Link/unlink own identities, my devices | – | ✅ own | ✅ own | ✅ own |
| View club queue / decide | – | ❌ | ✅ own club | ✅ |
| Link request → member | – | ❌ | ✅ club-managed only (R4) | ✅ any |
| Escalate | – | ❌ | ✅ | – |
| Platform queue | – | ❌ | ❌ | ✅ |
| Create member manually | – | ❌ | ✅ own club; admin role only with `club.roles.manage` (unchanged foundation rule) | ✅ |
| Import CSV / approve request | – | ❌ | ✅ own club, non-admin role only | ✅ |
| Change a member's email | – | own (R12) | club-managed only | ✅ |
| View a member's login methods (provider names, last use) | – | ❌ | ✅ own club members | ✅ |
| Unlink a member's identity | – | ❌ | club-managed only (n/a in practice) | ✅ |
| Rotate join code | – | ❌ | ✅ | ✅ |
| Revoke a user's sessions | – | own | ❌ | ✅ |

Rules:
- **Tenant isolation**: club-scoped routes keep `/clubs/{slug}/…` + `Tenant`
  extractor. Requests, mappings and runs are filtered by `ctx.club_id`. A
  request id from club B returns **404** in club A.
- **Impersonation**: intake actions require a **real** principal
  (`require_real`), the same rule as other authorization-sensitive actions.
  Identity linking and unlinking are refused while impersonating.

## 7. Database (new migration `m2026111x_000004_auth_onboarding`)

Additive first, destructive at the end of the same migration. Prod data is
tiny today, so a single migration is safe. It is still written forward-only,
with data backfills.

```
users  (altered)
  email             text NULL  (was NOT NULL)   -- UNIQUE kept; NULLs allowed
                                                --  CHECK (email = lower(email)) kept
  activated_at      timestamptz NULL            -- backfill: active users → created_at
  email_verified_at timestamptz NULL            -- backfill: activated users → activated_at
  status            CHECK (status IN ('active','disabled'))   -- 'invited' → 'active'

login_identities  (replaces user_oidc_identities; rows migrated)
  id uuid PK
  user_id uuid NOT NULL → users ON DELETE CASCADE
  provider text NOT NULL CHECK (provider IN ('google','microsoft','apple','facebook'))
  subject text NOT NULL
  email_at_link text NULL
  created_at timestamptz NOT NULL, last_used_at timestamptz NULL
  UNIQUE (provider, subject)
  INDEX (user_id)

oidc_flows
  id uuid PK
  state_hash bytea UNIQUE NOT NULL
  provider text NOT NULL, client text NOT NULL CHECK (client IN ('web','ios'))
  purpose text NOT NULL CHECK (purpose IN ('login','link'))
  nonce text NOT NULL, pkce_verifier text NOT NULL
  link_user_id uuid NULL → users ON DELETE CASCADE      -- purpose = link
  join_club_id uuid NULL → clubs ON DELETE CASCADE      -- came through a join code
  claims jsonb NULL                                     -- set after a verified callback
  registration_token_hash bytea NULL UNIQUE             -- NeedsRegistration outcome
  expires_at timestamptz NOT NULL                       -- 10 min (30 min once claims set)
  created_at timestamptz NOT NULL
  INDEX (expires_at)                                    -- purge

exchange_codes                               -- one-time code → session at exchange time
  code_hash bytea PK
  user_id uuid NOT NULL → users ON DELETE CASCADE
  client_label text NULL
  expires_at timestamptz NOT NULL               -- 60 s
  consumed_at timestamptz NULL

login_magic_links  (altered)
  purpose text NOT NULL DEFAULT 'login'
      CHECK (purpose IN ('login','confirm_link','register','confirm_email'))
  user_id uuid NULL (was NOT NULL)           -- NULL for 'register'
  email text NOT NULL                        -- address the link was sent to
  payload jsonb NOT NULL DEFAULT '{}'        -- confirm_link: {provider,subject,email}
                                             -- confirm_email: {new_email}
  CHECK ((purpose = 'register') = (user_id IS NULL))

auth_sessions  (altered)
  client_label text NULL

DROP user_passwords, invitations
(club_role_permissions: UPDATE permission 'club.invitations.manage' → 'club.intake.manage')

-- onboarding
club_join_codes
  club_id uuid PK → clubs ON DELETE CASCADE
  code text UNIQUE NOT NULL                  -- 10 chars, unambiguous alphabet
  enabled bool NOT NULL DEFAULT true
  rotated_at timestamptz NOT NULL

access_requests
  id uuid PK
  club_id uuid NULL → clubs ON DELETE CASCADE           -- NULL = platform queue
  provider text NULL, subject text NULL                 -- OIDC origin
  email text NULL, email_trusted bool NOT NULL          -- provider email or verified email
  CHECK ((provider IS NULL) = (subject IS NULL))
  CHECK (provider IS NOT NULL OR (email IS NOT NULL AND email_trusted))
  first_name text NOT NULL, last_name text NOT NULL, birthdate date NOT NULL
  message text NULL CHECK (length(message) <= 500)
  status text NOT NULL CHECK (status IN ('pending','approved','rejected'))
  escalated_at timestamptz NULL
  decided_by_user_id uuid NULL → users ON DELETE SET NULL
  decided_at timestamptz NULL, decision_reason text NULL
  resolved_user_id uuid NULL → users ON DELETE SET NULL
  created_at, updated_at timestamptz NOT NULL
  UNIQUE (provider, subject) WHERE status = 'pending'   -- partial (justified raw DDL, as before)
  UNIQUE (email) WHERE status = 'pending' AND provider IS NULL
  INDEX (club_id, status, created_at)
  INDEX (status, escalated_at) WHERE status = 'pending' -- platform queue

member_import_mappings
  club_id uuid PK → clubs ON DELETE CASCADE
  mapping jsonb NOT NULL        -- {"last_name":"NOM","first_name":"Prénom",…}
  date_format text NOT NULL CHECK (date_format IN ('dd/mm/yyyy','yyyy-mm-dd','mm/dd/yyyy'))
  updated_at timestamptz NOT NULL

member_import_runs
  id uuid PK
  club_id uuid NOT NULL → clubs ON DELETE CASCADE
  actor_user_id uuid NOT NULL → users
  file_name text NOT NULL, row_count int NOT NULL
  created_count int, linked_count int, skipped_count int, rejected_count int
  outcomes jsonb NOT NULL        -- [{row, status, user_id?, message?}], ≤ 5000 rows
  created_at timestamptz NOT NULL
  INDEX (club_id, created_at)

user_profiles  (users module, add index only)
  INDEX (licence_number) WHERE licence_number IS NOT NULL
  INDEX (birthdate)                                     -- suggestions + import match
```

The session row (and its plaintext token) is created only by `POST
/auth/exchange`, never at the callback: a leaked exchange code is worthless
after 60 s or one use, and no session token ever travels in a URL.

**Uniqueness summary**:

| Field | Scope |
|---|---|
| `users.email` | global, case-insensitive |
| `(provider, subject)` | global |
| join code | global |
| pending request per identity | global |
| import mapping | per club |
| `licence_number` | not unique: data quality is unknown; ambiguity leads to a "check" row |

**Deletion**:
- deleting a user cascades to identities, sessions and links; requests keep
  their audit trail through SET NULL;
- deleting a club cascades to join codes, requests, mappings and runs.

## 8. API

Public (`/api/v1/auth/*`):
```
GET  /auth/providers                         → [{id, label}] enabled in config
GET  /auth/oidc/{provider}/start?client=web|ios&purpose=login&join=CODE
                                             → 302 to provider (state, nonce, PKCE S256)
GET|POST /auth/oidc/{provider}/callback      → (POST = Apple form_post)
     → 302 to {base_url}/auth/done?code=X           (web)
       or fr.divemanager.app://auth/done?code=X     (ios)
       or {base_url}/auth/register?t=REG            (NeedsRegistration)
       or {base_url}/auth/pending | /auth/check-email
POST /auth/exchange {code}                   → {session_token, me}
POST /auth/apple/native {id_token, nonce, first_name?, last_name?}   (slice 5, iOS sheet)
POST /auth/magic-link {email}                → generic 200 (R9)
POST /auth/magic-link/consume {token}        → login: {session_token, me}
                                               confirm_link: links + {session_token, me}
                                               register: {registration_token}
                                               confirm_email: updates email, 200
GET  /auth/register/{registration_token}     → prefill {first_name,last_name,email,club?}
POST /auth/register {registration_token, first_name, last_name, birthdate, message?, join_code?}
                                             → 202 "request sent"
GET  /auth/join/{code}                       → {club_name} (404 if unknown/disabled)
```

Authenticated:
```
GET    /me/login-methods                     → {email?, identities:[{id,provider,email,last_used_at}]}
GET    /auth/oidc/{provider}/start?purpose=link   (requires session; binds flow to me)
DELETE /me/login-methods/{identity_id}       → 409 if last method (R11)
PUT    /me/email {email}                     → confirmation link to the new address (R12)
GET    /me/sessions                          → [{id, client_label, created_at, last_seen_at, current}]
DELETE /me/sessions/{id}                     → 404 if not mine
POST   /auth/logout, /auth/logout-all        (existing)
```

Club (`/clubs/{slug}/…`, `club.intake.manage`, real principal for writes):
```
GET  /access-requests?status=pending&page    → items + suggestions[] (≤5 each)
POST /access-requests/{id}/link {user_id}    → 403 if not club-managed
POST /access-requests/{id}/create {role_id, phone?, licence_number?}
POST /access-requests/{id}/reject {reason?}
POST /access-requests/{id}/escalate
GET  /join-code                              → {code, enabled, url}
POST /join-code/rotate ; PUT /join-code {enabled}
POST /members {first_name,last_name,email?,birthdate?,phone?,licence_number?,role_id}
     → same engine as one import row; returns {status: created|linked|check, …}
PUT  /members/{user_id}/email {email}        → club-managed only
GET  /members/{user_id}/login-methods        → provider names + last use (no subjects)
POST /imports/members/inspect   (multipart file)        → {encoding, separator, headers,
                                  sample_rows[5], suggested_mapping, date_format_guess}
POST /imports/members/preview   (multipart file + mapping + date_format + role_id)
                                → {rows:[{n, status: new|update|check|rejected,
                                   match?, diffs[], errors[]}], counts}
POST /imports/members/commit    (multipart file + mapping + date_format + role_id
                                 + decisions:{row_n: same_person|new|skip}
                                 + send_welcome: bool)  → import_run summary
GET  /imports/members/runs?page                       → past runs
```

Platform (`/platform/…`, real global admin):
```
GET  /platform/access-requests?status&escalated&page
POST /platform/access-requests/{id}/link|create|reject   (create takes club_id + role_id, or none)
POST /platform/clubs/{id}/admins {email, first_name, last_name}
     → find-or-create user + Administrateur membership + welcome email (replaces invite_club_admin)
POST /platform/users/{id}/revoke-sessions
GET  /platform/users/{id}/login-methods ; DELETE /platform/users/{id}/login-methods/{identity_id}
PUT  /platform/users/{id}/email
```

**Removed endpoints**: `/auth/login`, `/auth/login-methods`, `/me/password`,
`/auth/invitations/*`, `/clubs/{slug}/invitations*`,
`/platform/clubs/{id}/invitations`, `/clubs/{slug}/memberships` POST.
Adding a member is now `/clubs/{slug}/members` POST.

**Stateless import**: the file is re-uploaded on preview and commit, so
nothing is stored server-side. Limits: 2 MiB, 5,000 rows, 50 columns. The
server is the only parser: encoding detection (UTF-8 BOM, UTF-8, otherwise
Windows-1252), separator sniffing (`;`, `,`, tab), header row = first
non-empty row.

## 9. Errors

New `AppError` mappings (generic messages, nothing internal leaks):

| Status | Case |
|---|---|
| 400 | `oidc_state_invalid`, `oidc_expired`, `oidc_token_invalid` (signature, iss, aud, exp, nonce) |
| 410 | `exchange_code_used`, `link_expired` |
| 403 | `account_disabled` |
| 409 | `identity_already_linked`, `last_login_method`, `request_already_pending`, `email_in_use` |
| 403 | `not_club_managed` (with a hint "escalate to the platform") |
| 422 | `import_*` (size, rows, encoding, unmapped required field, invalid date format), with per-row errors in the payload, not as HTTP errors |
| 429 | `rate_limited`, used on the register and join endpoints only; the magic-link endpoint stays generic |

## 10. Performance

- Scale: ~10 clubs and a few thousand users in year 1.
- Login: one point query on `login_identities(provider, subject)`, plus at
  most one on `users.email`.
- Import preview for N rows: **3 batched lookups**, never one per row:
  - users by `email IN (…)`;
  - profiles by `licence_number IN (…)`;
  - users joined with profiles by `birthdate IN (…)`, then names compared in
    memory.

  Then a single read of the club's memberships. Commit: bulk inserts in one
  transaction. 5,000 rows is fine on the VPS.
- Queue suggestions: one query per page, not per request. Candidates are the
  club-managed members whose `birthdate` is in the page's birthdates, plus a
  name prefilter; the ranking happens in memory.
- Purge task (existing interval): expired `oidc_flows`, `exchange_codes`,
  magic links, and decided requests older than 90 days.
- JWKS and discovery documents are cached in memory, refreshed on an unknown
  `kid` with a rate limit.

## 11. Security notes

- OIDC: authorization code + **PKCE S256 + nonce + state**. The id_token is
  verified locally against the JWKS (`iss`, `aud`, `exp`, `iat` skew, `nonce`).
  - **Microsoft `common` endpoint**: the issuer is tenant-templated, so it is
    validated as `https://login.microsoftonline.com/{tid}/v2.0` with `tid`
    read from the token.
  - **Apple**: `client_secret` is a short-lived ES256 JWT signed with the
    Apple key; the name arrives only on the first authorization (stored on
    the flow).
- **No token in URLs** except single-use, short-lived codes (exchange 60 s,
  magic links 12 h, registration 30 min).
- Deep link (iOS): exchange codes are useless without the app's call within
  60 s and are single-use. Use a **Universal Link** in production (custom
  scheme only as a dev fallback) to prevent scheme hijacking.
- Enumeration: magic-link and register paths are generic (R9). Import and
  manual creation do reveal "an account exists with this email" to club
  admins; this is accepted (admin role, audited) and already true today.
- **Takeover vectors closed by R4/R12**:
  - a club admin cannot change the email of a shared or active user;
  - a club admin cannot link an identity to them.
- Rate limits:
  - magic link: 3 per email and 20 per IP, per 15 min (existing);
  - register: 5 per IP per hour;
  - OIDC start: 30 per IP per 15 min;
  - join code lookup: 30 per IP per 15 min.
- Audit events:
  - `identity.auto_linked`, `identity.linked`, `identity.unlinked`;
  - `email.changed`;
  - `access_request.created`, `access_request.decided`,
    `access_request.escalated`;
  - `import.committed`, `member.created`;
  - `join_code.rotated`, `sessions.revoked`.

## 12. Testing

**Domain/unit**:
- trust matrix (R7): Google verified/unverified, MS personal/work tenant,
  Apple;
- CSV: encoding detection (UTF-8 BOM, CP1252 accents), separator sniffing,
  date parsing per format, header synonym suggestion;
- name normalization (accents, case, hyphens);
- `is_club_managed`: activated, two clubs, global admin.

**Integration (HTTP, real DB, fake OIDC provider)**. The fake provider is an
in-test JWKS plus a signer; there are no network calls.
- Login resolution R8, cases 1 to 5:
  - a known subject wins over a different email;
  - a disabled user is refused;
  - a pending request gives the pending screen;
  - a trusted email auto-links and sets `activated_at`;
  - an untrusted email gets a confirmation link, and the click links the
    identity;
  - an unknown identity gets a registration flow, and submitting creates a
    request.
- OIDC hardening:
  - bad state, bad nonce or a reused state are refused;
  - an expired flow is refused;
  - a forged signature or a wrong `aud`/`iss` is refused;
  - a replayed exchange code returns 410.
- Magic link:
  - the generic response is identical for known and unknown emails;
  - an unknown email receives a registration link;
  - every purpose is single-use;
  - `confirm_email` changes the email only on click.
- Link/unlink from profile:
  - an identity already linked to another user returns 409;
  - removing the last method returns 409;
  - linking is refused while impersonating.
- Access requests:
  - a club admin can link to a club-managed member;
  - **403** for a member of another club too, for an activated member, and
    for a global admin;
  - escalation makes the request visible in the platform queue;
  - a platform admin can link to anyone;
  - a duplicate pending request returns 409;
  - a request is auto-resolved when the identity gets linked through login;
  - a rotated join code makes the old code 404.
- **Tenant isolation**:
  - club A's admin cannot list, decide or read requests, runs or the join
    code of club B (404);
  - users in several clubs are never club-managed.
- Import:
  - the mapping is remembered;
  - preview performs no writes;
  - a commit re-validates and rejects a tampered decision (a "same person"
    decision pointing to a row that was not "check");
  - transaction rollback on an injected failure;
  - fill-empty-only: an existing user in club B is never overwritten and only
    gets a membership in A;
  - duplicates inside the file;
  - ambiguous licence numbers;
  - an admin role is refused;
  - size and row limits.
- Email change:
  - a club admin can change it on a club-managed member;
  - 403 on a shared or active member.
- Sessions:
  - "my devices" lists only mine;
  - deleting someone else's session id returns 404;
  - a platform revoke takes effect immediately.
- Migration test: a DB seeded with the old schema (invited user, password,
  invitation, `club.invitations.manage` grant) migrates to the expected rows.

**Frontend**:
- login screen with buttons from `/auth/providers`;
- the register and pending screens;
- the mapping step of the import wizard, with suggestions;
- the queue card showing suggestions.

## 13. Implementation plan (6 shippable slices, each reviewed and deployed)

1. **Simplify**: remove passwords and invitations.
   - Migration part 1: email nullable, `activated_at`, status, permission
     rename, drop the tables.
   - New `POST /clubs/{slug}/members` (single-row engine) and the platform
     "add club admin" endpoint.
   - Login screen reduced to the magic link; frontend pages removed or
     replaced (AcceptInvitation, the Invitations tab, the password forms).
2. **Google + Microsoft**:
   - `login_identities`, `oidc_flows`, `exchange_codes`;
   - resolution R8 (cases 1, 3, 4; case 5 shows "contact your club" until
     slice 3);
   - `confirm_link` magic links;
   - profile login methods (link/unlink, change email);
   - "my devices";
   - redirect URIs and app registrations (product owner: Google Cloud +
     Entra apps).
3. **Access requests**: join codes (link + QR), the register flow (OIDC and
   magic link), club and platform queues with suggestions, escalation,
   decision emails, the 90-day purge.
4. **CSV import**: inspect, preview and commit, remembered mapping, the
   wizard UI, welcome emails (background sending, throttled to SMTP limits),
   run history.
5. **Apple + iOS**:
   - Sign in with Apple (web and native sheet);
   - Capacitor flow with the system browser (ASWebAuthenticationSession) and
     Universal Link `/v2/auth/done`;
   - `client=ios` redirects;
   - App Store review checks (guideline 4.8).
6. **Passkeys**: a separate short design (WebAuthn via `webauthn-rs`);
   enrolment is offered after login.

New crates, to justify at review:
- `openidconnect` (discovery, PKCE, JWKS validation);
- `csv`, `encoding_rs`, `chardetng`;
- `jsonwebtoken` (Apple client secret);
- `strsim` or a hand-written normalization, for names.

## 14. ADR-0009 (proposed) — Admin-curated accounts, linked login identities, no passwords

**Status**: proposed (to accept with this design).
**Supersedes**:
- ADR-0004 §1 (invitation-only through invitations);
- ADR-0004 §3 (three methods including passwords);
- ADR-0004 "OIDC first link requires invited/existing account".

**Decision**:
- Accounts are admin-curated (import, manual creation, approval of an access
  request) instead of invitation tokens.
- Login methods are magic links to the account email plus OIDC identities
  linked by `(provider, subject)`; passwords are removed.
- First links rely on provider-trusted emails, or on an emailed confirmation
  otherwise.
- Unknown people enter through access requests reviewed by club admins, who
  may only link "club-managed" accounts (never logged in, single club); the
  rest is escalated to platform admins.

**Trade-offs**:
- Admins carry a review queue (mitigated by auto-linking and suggestions).
- Account existence is visible to club admins during import (accepted).
- OIDC provider setup is a product-owner task (Google, Microsoft, Apple
  developer accounts).

## 15. Open questions — settled (product owner, 2026-11-16: recommendations accepted)

- **Q1 — Shared family emails.** `users.email` stays unique. In a CSV, a
  second person with an already-used email becomes a "check" row. The admin
  then either imports that person **without an email**, so they reach the
  account through the queue or an SSO login (children usually never log in),
  or fixes the address. *Recommended: accept for v1.* A true "family account"
  would need its own design.
- **Q2 — Join codes.** One rotatable code per club, distributed as a link and
  a QR code. Platform-wide requests (no club) are allowed and go to platform
  admins. *Recommended: yes.*
- **Q3 — Birthdate required in access requests?** It is what makes the
  suggestions reliable. *Recommended: yes.*
- **Q4 — Purge decided requests after 90 days?** *Recommended: yes* (data
  minimization).
- **Q5 — May club admins see which providers a member uses** (e.g. "Google,
  last used 3 days ago"; never the account id)? It helps support.
  *Recommended: yes.*
- **Q6 — Welcome email**: opt-in checkbox at import commit, no token inside.
  *Recommended: yes.*
