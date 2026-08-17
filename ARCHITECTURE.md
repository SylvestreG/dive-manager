# Architecture

Settled principles plus open decisions. Detailed rationale lives in
`DECISIONS.md`.

## Shape

- **Modular monolith** in stable Rust: one deployable binary, internal module
  boundaries per `MODULES.md`. No microservices, no message broker, no cache
  server unless a measured need appears.
- **Single PostgreSQL database** for all clubs. Club-owned tables carry
  `club_id` (ADR-0002, proposed). Migrations are ordered, reviewed, and
  runnable independently of app boot (Fosse migrates automatically at startup —
  fine for dev, explicit for prod).
- Target deployment: one small VPS — binary + PostgreSQL + reverse proxy.

## Layering (per module)

```
HTTP handlers  →  domain services  →  repositories  →  PostgreSQL
      ↓ auth/tenant extraction           ↑ mandatory TenantContext parameter
```

- Handlers do: extract auth + tenant context, validate input shape, call the
  domain, map to response DTOs. No business logic, no queries.
- Domain services own business rules — exactly one implementation per rule
  (Fosse's rule drift came from duplication).
- Repositories take a `TenantContext` for club-owned data; there is no API to
  query club-owned tables without one. Tenant isolation is enforced by
  construction and verified by tests.

## Cross-cutting rules

### Tenancy
- Tenant = club. The tenant context derives from the authenticated user's
  membership and the club targeted by the request — never from a raw client-
  supplied club id without a membership check.
- Users are global (one login, one email); memberships bind them to clubs.

### Authorization
- Explicit grants only; absence of data denies (ADR-0003). Roles: global
  admin (platform attribute), club admin, member — extensible per-club
  permissions on top, modeled after Fosse's 22-permission catalogue.
- Every protected handler states its required permission; a routing-level
  audit test asserts no unprotected mutation routes exist (Fosse's main hole).
- Public token endpoints (magic links, recaps) are scoped to exactly the data
  the token was minted for.

### Security baseline
- Argon2 passwords, rate-limited login, single-use time-limited tokens,
  strict CORS, HTML escaping in every template, no secrets in the repo,
  audit log for sensitive actions (validation changes, impersonation, role
  changes).
- Impersonation (kept from Fosse): explicit, logged, permissions of the
  target, "real admin" required for destructive actions.

### Errors, config, observability
- Typed error enum per module mapped to consistent HTTP problem responses;
  never leak internals in `details`; never swallow the underlying error in
  logs (Fosse does both).
- Config from a file **with env-var overrides**; missing optional integrations
  (SMTP) degrade features, not boot.
- `tracing` with request/tenant correlation.

### Performance
- Pagination on every list endpoint; no N+1 (Fosse's palanquée tree does one
  query per row — load trees with joins/batched queries); transactions around
  multi-row writes (sortie creation, imports, permission updates).
- Payloads sized for mobile networks; expensive aggregates (gear accounting)
  computed server-side once, not re-derived by clients.

## Settled stack decisions

| Topic | Decision |
|---|---|
| HTTP framework | **Axum** (ADR-0006) |
| DB layer | **SeaORM** for migrations and all queries; **no raw SQL** outside DSL-impossible migration statements (ADR-0006, strong project rule) |
| Auth | Invitation-only; OIDC (Google at launch) + login magic links (12 h) + optional passwords; revocable server-side sessions, 30 d sliding / 90 d max (ADR-0004) |
| Frontend | React + TS + Vite PWA, Capacitor for stores, mobile-first review rule (ADR-0005) |

## Still open

| Topic | Note |
|---|---|
| RLS as defense-in-depth | Later, on top of ADR-0002; not needed for v1 |
| Microsoft OIDC | Post-launch (config + Azure app registration) |
