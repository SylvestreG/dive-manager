# Architecture Decision Records

Statuses: **accepted** (settled), **proposed** (awaiting discussion), **superseded**.

---

## ADR-0001 — Rewrite Fosse as a multi-tenant modular monolith in Rust

**Status:** accepted (project charter, 2026-08-17)

**Problem.** Fosse works for one club but its architecture assumes a single
tenant throughout (config-file admin list, global email uniqueness, implicit
permission fallbacks). Extending it in place would spread tenant checks across
code that was never designed for them.

**Decision.** Full rewrite. Backend in stable Rust as a modular monolith.
Single PostgreSQL database shared by all clubs. No microservices, no Redis /
Kafka / Elasticsearch / Kubernetes unless a real requirement appears.

**Consequences.** Fosse remains the functional reference and stays runnable
during the rewrite. Every module is rebuilt with tenant context from the start.
Deployment target stays a small VPS.

---

## ADR-0002 — Tenant model: shared tables with a `club_id` discriminator

**Status:** accepted (product owner, 2026-08-17)

**Problem.** "Single database for all clubs" still leaves a choice:
schema-per-club or shared tables with a tenant column.

**Options.**
1. Shared tables, every club-owned row carries `club_id` (FK to `clubs`).
2. One PostgreSQL schema per club.
3. Row-level security (RLS) on top of option 1.

**Recommendation.** Option 1. Schema-per-club multiplies migrations and
connections and breaks cross-club queries (a user belonging to several clubs is
a core requirement). RLS can be layered on later as defense in depth without
changing the data model; it is not required for v1 as long as every repository
query is tenant-scoped by construction (enforced at the query-layer API, tested
by tenant-isolation tests).

**Trade-offs.** Requires discipline: no query on club-owned tables without a
tenant filter. Mitigation: repository layer takes a mandatory tenant context
parameter; handlers never build raw queries.

---

## ADR-0003 — Do not reproduce Fosse's implicit admin fallbacks

**Status:** accepted (security)

**Problem.** Fosse's ACL middleware (`backend/src/middleware/acl.rs`) grants
`Permission::all()` when the authenticated email is missing from the `people`
table, and again when the person has no group ("legacy compatibility"). This is
an implicit privilege escalation path relying on the assumption that only
config-listed admins can obtain a JWT.

**Decision.** In Dive Manager, permissions are always explicit grants. An
authenticated user with no membership/role has **no** permissions. Global
admins are explicit records, not a config-file email list. Absence of data
never grants access.

---

## ADR-0004 — Authentication and account model

**Status:** accepted (product owner, 2026-08-17)

**Problem.** Fosse mixes four auth methods with implicit rules (Gmail domain ⇒
forced Google login), a config-file admin list frozen into a 24 h irrevocable
JWT, no rate limiting, and tokens resolved by person instead of by resource.
Dive Manager needs invitation-controlled, multi-club, revocable auth.

**Decision.**

1. **Invitation-only platform.** No open signup. Global admins are
   bootstrapped from config **only when no global admin exists in the
   database**; afterwards the database is the sole source of truth and the
   config list is ignored. Club admins and members enter via stored,
   expiring, single-use invitations (who invited whom is audited). CSV import
   creates member profiles and invitations in bulk.
2. **Member profiles can exist without an activated user account.** A club can
   register, import and enroll a member (and send them questionnaire magic
   links) before that person ever logs in. Activation links the profile to a
   user account by verified email.
3. **Three login methods, cumulable on one account:**
   - **OIDC** (generic OpenID Connect; Google and Microsoft at launch),
     configured at **platform level**. Providers are configuration, not code.
     Identities are linked by `(provider, subject)`, never by email alone;
     first link requires a provider-verified email matching an invited or
     existing account. No French consumer ISP (Orange/Free/SFR) offers usable
     public OIDC — those users use the other methods.
   - **Login magic links** by email (the default path; distinct from
     questionnaire tokens).
   - **Password** (optional per user, Argon2, rate-limited).
   The login screen offers all methods; the method is **never inferred from
   the email domain**.
4. **Server-side revocable sessions** (opaque token, hashed at rest) instead
   of self-contained JWTs. Role changes take effect immediately. Suits a
   single-VPS modular monolith; no shared-nothing scaling requirement exists.

**Trade-offs.** A DB lookup per request (acceptable at our scale; cacheable
in-process later). Running our own OIDC linking logic requires care
(`email_verified`, nonce, PKCE) — specified in the foundation design.

**Consequences.** Fosse's Google-only-for-Gmail UX disappears; USI users
migrate to invitation-based activation. `identity` owns users, credentials,
sessions, invitations; `tenancy` owns clubs, memberships, roles.

---

## ADR-0005 — Frontend: React + TypeScript + Vite PWA, wrapped with Capacitor for the stores

**Status:** accepted (product owner, 2026-08-17)

**Problem.** Charter requires web, mobile web, iOS and Android. Product owner
requires **App Store / Play Store presence by mid-September 2026** and full USI
cutover in September, with mobile-first as the primary goal.

**Options.** (1) Mobile-first web PWA now, native wrapper later; (2) Flutter;
(3) React Native/Expo; (4) Rust SSR + HTMX.

**Decision.** One codebase: **React + TypeScript + Vite**, mobile-first,
wrapped with **Capacitor from day one** to produce the iOS/Android apps.
Flutter/RN are incompatible with the deadline and Flutter's web output is poor
for this form-and-table-heavy app; SSR+HTMX would slow down the interactive
builders. React over Svelte/Vue for ecosystem depth (drag & drop, PDF
rendering, tables) and long-term viability.

**Store v1 scope = the member app**: login (OIDC/magic link/password), my
dives, answer questionnaires, my competencies, my profile. Admin and DP tools
ship on the responsive web (same codebase) — reduces Apple guideline-4.2
rejection risk and matches real usage.

**Trade-offs.** Not fully native UX; push notifications and deep offline
deferred. Store logistics (Apple D-U-N-S, Google Play organization account,
review cycles) sit on the critical path and are the product owner's to start
immediately.

**Consequences.** Mobile-first is enforced as a review rule: every screen is
designed and validated at phone size first (`DEVELOPMENT.md`). Store
submission target: 2026-09-05/08 to buffer review rejections.

---

## ADR-0006 — Backend stack: Axum + SeaORM, no raw SQL

**Status:** accepted (product owner, 2026-08-17)

**Problem.** The HTTP framework and database layer were the last open
foundation choices.

**Decision.** **Axum** for HTTP (continuity with Fosse, mainstream, tower
ecosystem). **SeaORM for migrations and all queries** — product owner's strong
project-wide rule: **no raw SQL anywhere it can be avoided**.

Allowed exceptions, each requiring a code-review justification:
1. Migration statements the SeaORM DSL cannot express (e.g. `CREATE EXTENSION
   citext`, CHECK constraints, partial indexes) — confined to migration files.
2. Nothing else. Runtime query paths are SeaORM only.

**Trade-offs.** Some complex reads (palanquée trees, aggregates) are less
natural in an ORM; the answer is SeaORM's query builder with joins and
`select_only` projections — not raw SQL, and not N+1 loops (Fosse's SeaORM
usage degenerated into per-row queries; reviews must reject that pattern).

**Consequences.** Entities generated/maintained per module; migrations are
SeaORM migration crates run via a dedicated command (and at boot in dev).
