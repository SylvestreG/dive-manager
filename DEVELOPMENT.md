# Development

## Layout

```
backend/
  migration/            SeaORM migration crate (run via `dive-manager migrate`)
  src/
    <module>/           one directory per module (identity, tenancy, …)
      entities/         SeaORM models owned by the module
      services/         domain logic + data access (SeaORM only)
      handlers/         HTTP: extract, validate shape, call services, map DTOs
      tests/            integration tests, one file per theme
    test_support.rs     shared test harness (cfg(test))
    app.rs              router; config.rs, errors.rs, mailer.rs, … shared infra
```

- Foundation modules use services for data access. Business (club-scoped)
  modules add a `repositories/` layer whose functions take the mandatory
  `TenantContext` — that is where the tenant-filter-by-construction rule is
  enforced.
- Cross-module access goes through the other module's services, never its
  tables.

## Setup — backend

- Rust stable (rustup), PostgreSQL 16, cargo-nextest.
- `cp config.example.json config.json`, adjust `database.url`;
  set `base_url` to the FRONTEND origin (magic links point there).
- `cargo run -- migrate` applies migrations (or `database.migrate_on_boot:
  true` in dev). `cargo run` serves (port 8081 in the example config).
- No SMTP configured ⇒ emails are **logged in the backend console** (dev
  mode) — copy the magic-link / invitation URLs from there.

## SSO (Google / Microsoft / Apple) — optional in dev

Providers are configured in `config.json` → `oidc` (absent = button hidden):

```json
"oidc": {
  "google":    { "client_id": "…", "client_secret": "…" },
  "microsoft": { "client_id": "…", "client_secret": "…" },
  "apple": { "client_id": "fr.divemanager.web", "team_id": "…", "key_id": "…",
             "private_key_path": "/path/to/AuthKey.p8" }
}
```

Redirect URI to register at the provider: `{base_url}/api/v1/auth/oidc/{provider}/callback`
(dev: `http://localhost:5173/api/v1/auth/oidc/google/callback` through the
Vite proxy; Apple refuses localhost — test Apple on the deployed instance).
Integration tests use a local fake provider (`identity/tests/oidc.rs`); no
real provider is ever called by the test suite.

## Setup — frontend (`frontend/`, React + Vite + Capacitor, ADR-0005)

- Node 22 (`nix shell nixpkgs#nodejs_22` works). `npm install`.
- **Web**: `npm run dev` → http://localhost:5173 (proxies `/api` to the
  backend on 8081). First login: request a magic link, copy the URL from the
  backend console.
- **iPhone simulator** (one-time machine setup, requires ~15 min):
  1. Install Xcode from the App Store, then
     `sudo xcode-select -s /Applications/Xcode.app`.
  2. Install CocoaPods (`brew install cocoapods`).
  3. `npm run ios` — builds with `VITE_API_URL=http://localhost:8081/api/v1`
     (the simulator shares the host loopback; ATS exempts localhost), syncs
     Capacitor and opens Xcode → pick an iPhone simulator → Run.
- CORS: the backend config must allow `http://localhost:5173` and
  `capacitor://localhost` (see `config.example.json`).

## Tests

- `cargo nextest run` is the standard runner.
- Integration tests need `TEST_DATABASE_URL` pointing at a PostgreSQL
  **server**; each test creates its own throwaway database (parallel-safe).
  Local one-liner without installing Postgres:
  `nix shell nixpkgs#postgresql_16 -c pg_ctl …` (see CI for the reference
  setup; any Postgres 16 works).
- Every module ships negative tests: what users must NOT be able to do
  (cross-club IDOR, missing permissions, impersonation boundaries).

## Conventions (settled)

- **Mobile-first is a review rule**: every screen is designed and validated at
  phone size first; desktop is the adaptation. Frontend: React + TS + Vite
  PWA wrapped with Capacitor (ADR-0005).
- Stable Rust; `cargo fmt --check` and `cargo clippy --all-targets -D warnings`
  gate CI.
- **SeaORM for migrations and every query; no raw SQL** in runtime paths —
  the only tolerated raw statements are DSL-impossible migration DDL and
  test-infrastructure DDL (throwaway test databases), justified in review
  (ADR-0006). N+1 per-row query loops are rejected in review; use
  joins/batched queries.
- All code, schema, API and docs in English. User-facing copy is French and
  lives in dedicated template/text modules — never inline in logic.
- Emails/slugs stored lowercase, enforced by CHECK constraints; secrets
  (session tokens, magic links, invitations) stored hashed (SHA-256),
  passwords Argon2id.
- Migrations: real dates in names, never edited after deployment, runnable via
  the `migrate` subcommand.

## Definition of Done (per module)

- [ ] Business rules implemented — once, in the domain layer
- [ ] Authorization implemented and every route's permission stated
- [ ] Tenant isolation verified by tests (cross-club access attempts fail)
- [ ] Negative tests: what users must NOT be able to do
- [ ] DB constraints and indexes match the documented uniqueness/query patterns
- [ ] No N+1 on hot paths; lists paginated; multi-row writes transactional
- [ ] Edge cases from `DOMAIN.md` covered or explicitly deferred
- [ ] Senior code review done; issues fixed
- [ ] `DOMAIN.md` / `MODULES.md` / `DECISIONS.md` updated
- [ ] No known critical issue
