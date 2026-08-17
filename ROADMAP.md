# Roadmap

**Milestone 1 — iOS + Android apps on the stores by mid-September 2026**
(product owner, 2026-08-17). Submission target 2026-09-05/08. The store v1 may
ship with **base modules only** (login, profile, levels/competencies view);
dives/questionnaires follow as app updates. Store logistics (Apple D-U-N-S,
Google Play organization account) start immediately — they are on the
critical path.

**Milestone 2 — USI fully cuts over to Dive Manager in September 2026** (not
just a data import). Fosse stays untouched and available as the fallback; a
go/no-go review is held ~2026-09-20. The cutover scope is strictly limited —
see `docs/design/migration-strategy.md`.

Provisional module sequence. Order and boundaries will be confirmed after the
Fosse functional analysis lands in `DOMAIN.md` and `MODULES.md`.

## Phase 0 — Analysis & foundations design (current)

- [x] Full functional analysis of Fosse (backend + frontend) → `DOMAIN.md`, `docs/analysis/`
- [x] Module map derived from the analysis → `MODULES.md`
- [ ] Foundation module design: identity, clubs, memberships, roles/permissions
- [ ] Decide tenant model (ADR-0002) and auth approach with the product owner
- [ ] Repository/workspace skeleton, CI, `DEVELOPMENT.md`

## Phase 1 — Platform foundation module

Identity & tenancy: users, clubs, club memberships, roles (global admin, club
admin, member), permission checks, tenant context plumbing, audit logging
basics. Everything else depends on this; it gets the strictest review and the
first tenant-isolation test suite.

## Phase 2+ — Business modules (order to confirm after analysis)

Likely candidates, one at a time:

1. People / members (club roster, diving levels, licenses)
2. Activities: pool sessions & dive trips (sorties)
3. Registrations / questionnaires
4. Palanquées & rotations (incl. security sheet PDF)
5. Competencies & skill validations
6. Notifications / emails
7. Imports
8. Club configuration & module toggles (may move earlier if needed)

## Migration from Fosse

See `docs/design/migration-strategy.md`. Full cutover in September within a
strict scope; an idempotent importer re-syncs USI data until the final sync;
Fosse untouched = zero-cost rollback; go/no-go ~2026-09-20; post-cutover
backlog for everything non-essential.

## Major risks

1. **September deadline vs scope** — mitigated by limiting Phase A to import +
   consultation (no operational workflows).
2. **Business rules exist only in Fosse's frontend, duplicated and drifted** —
   risk of silently losing real behavior. Mitigation: functional inventory
   (`docs/analysis/functional-inventory.md`), parity checks on real data,
   Fosse kept runnable as the reference.
3. **Fosse has no usable test suite** — no executable spec; parity is verified
   against production data and club-admin walkthroughs.
4. **Fosse's known security holes stay in production until cutover** (public
   recap leaks magic links; fail-open ACL; unprotected mutation routes).
   Product owner decided (2026-08-17) **not to hotfix Fosse** — residual risk
   accepted, shortened by the September cutover.
5. **Data quality** — `diving_level` free-text blob, unreliable
   `comes_from_issoire`, blank locations. Mitigation: audit production data
   before freezing the importer; fail loudly on unknown values.
6. **Auth model change** (Google/temp-passwords → invitations/magic links) —
   member adoption risk; mitigation: OIDC keeps one-click Google login, magic
   links are simpler than passwords.
7. **Frontend stack undecided** — blocks Phase A's consultation UI if it
   slips; decide at workspace creation.

## Later / to evaluate

- Frontend stack selection (web + mobile web + iOS + Android) — dedicated
  comparison before any frontend code
- Data migration tooling from a live Fosse instance
- Payments, equipment, boats, statistics — only if real requirements emerge
