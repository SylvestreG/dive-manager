# Modules

Module map for the modular monolith, derived from the Fosse analysis.
Statuses: `planned` → `analyzing` → `designing` → `implementing` → `done`.

Modules are internal Rust modules (crate or module boundaries inside one
workspace), not plugins. A module owns its tables, its domain rules, its API
handlers and its permissions. Cross-module access goes through explicit Rust
APIs, never through another module's tables.

## Foundation (not club-toggleable)

### `identity` — status: **implemented** (`backend/src/identity/`; OIDC Google = next slice)
Users, authentication (login magic links + optional passwords; OIDC schema
ready, flow pending), revocable sessions, bootstrap, audit log. No club
knowledge. Design: `docs/design/foundation-module.md`.

### `tenancy` — status: **implemented** (`backend/src/tenancy/`)
Clubs, memberships (user ↔ club, with roles), club configuration (locations/
pools with behavior flags, departure point, gear conventions, branding, SMTP),
enabled-modules registry. Provides the **tenant context** every request must
carry and the authorization primitives (role/permission checks). Depends on
`identity`.

Design constraint from the Fosse analysis: permissions are explicit grants
only — an authenticated user with no membership has no access (Fosse is
fail-open; see ADR-0003).

## Business modules (club-toggleable where sensible)

### `members` — status: planned
Per-club member profiles: identity link, phone, structured diving levels
(FFESSM ladder), preparing level, default gear preferences. Level-cascade and
next-preparable-level rules live here. Depends on `tenancy`.
Fosse source: `people`, `diving_level` comma-blob (to be normalized).

### `activities` — status: planned
Pool sessions (fosses), sorties and their dives, dive-director assignments
(single mechanism). Location-driven behavior comes from club config, not
hardcoded strings. Depends on `members`.
Fosse source: `sessions`, `sorties`, `dive_directors`.

### `registrations` — status: planned
Registrations (questionnaires): gear requests, nitrox, transport/carpool,
submission state; magic-link tokens (correctly scoped, single-use); public
recap tokens (without data leaks); the registration business rules R1–R4 in
one place. Depends on `activities`.
Fosse source: `questionnaires`, `email_jobs` (token part).

### `palanquees` — status: planned
Rotations, palanquées, members, composition rules, gear accounting (single
server-side implementation), bilan (realised params), fiche de sécurité PDF.
Depends on `registrations`.
Fosse source: `rotations`, `palanquees`, `palanquee_members`,
`fiche_securite.rs`.

### `competencies` — status: planned
Per-club competency tree (level → domain → module → skill), validation stages,
skill validations **with history**, monotonic-progression rule, min-validator
gate, level documents (PDF templates + stamp positions + filled generation),
validation audit. Depends on `members`.
Fosse source: `competency_*`, `validation_stages`, `skill_validations`,
`level_documents`, `skill_document_positions`.

### `imports` — status: planned
CSV import of members into a session, per-row error reporting, idempotency.
Depends on `registrations`.
Fosse source: `import_jobs`, `services/import.rs`.

### `notifications` — status: planned
Email strategy (send vs assist — product decision pending), templates as club
configuration with proper HTML escaping, invitation + recap emails, job
tracking. Depends on `registrations`.
Fosse source: `email_jobs`, `services/email.rs`, frontend EmailsPage cockpit.

## Module dependency graph

```
identity ← tenancy ← members ← activities ← registrations ← palanquees
                          ↑                        ↑    ↑
                   competencies             imports    notifications
```

## Not planned (Fosse has none; add only on real demand)

Payments, equipment inventory, boats, compressors, attendance, generic
documents, statistics beyond what pages need.
