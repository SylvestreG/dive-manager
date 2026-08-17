# Migration strategy — Fosse → Dive Manager

Status: **accepted direction** (product owner, 2026-08-17): **full cutover of
USI to Dive Manager in September 2026** — Dive Manager becomes the daily tool,
not just a data mirror. Safety valve: Fosse is never modified, so the rollback
is simply "keep using Fosse"; a **go/no-go review is held ~2026-09-20** against
the cutover checklist below.

## Principles

1. **Fosse keeps running untouched** during the whole rewrite. Dive Manager is
   built alongside, module by module.
2. **Data flows one way, repeatedly**: an importer reads Fosse's PostgreSQL
   and writes into Dive Manager. It is **idempotent and re-runnable** (upsert
   by natural keys), so USI's data can be refreshed daily/weekly until cutover
   — "migration day" is a non-event, just the last sync.
3. **Cutover is per functional area**, not per date: an area switches when its
   Dive Manager module is done (Definition of Done) and its data verified.
4. Until full cutover, **Fosse is the write-master** for not-yet-migrated
   areas. No dual-write, no write-back from Dive Manager to Fosse — an area is
   either read-only-mirrored or fully cut over.

## The importer

A one-shot Rust CLI in the Dive Manager workspace (`tools/fosse-import`),
connecting to both databases. Not a module; throwaway code with tests on the
tricky mappings.

Key mappings (from the analysis):
- `people` → club **member profiles** (club = USI) + **users** only for those
  with a login history worth keeping; everyone else stays profile-without-
  account until invited (ADR-0004 §2). Emails normalized (citext).
- `diving_level` comma-blob → structured levels + preparing level. The parser
  must handle every variant found in production (audit the real data first;
  Fosse never validated writes to this field).
- `group_id` → membership role (Administrateur/Encadrant/Membre); config
  `admin.emails` → USI club admins + platform global admin(s) as decided.
- Competency tree, stages, positions, level-document blobs → per-club copies.
- `skill_validations` → current-state rows become the initial history entry
  (earlier history is unrecoverable — Fosse never kept it).
- Sessions/sorties/questionnaires/palanquées → imported for **history**
  (read-only consultation), with blank locations resolved to Coubertin and the
  location strings mapped to club-config locations.
- Known data-quality traps: `comes_from_issoire` default-true is unreliable
  (public submits never wrote it); `email_jobs.sortie_id` always NULL; tokens
  are NOT migrated (all Fosse tokens die at cutover).

## Cutover scope (must exist by the go/no-go)

Everything USI needs day to day, nothing more:

- Foundation (`identity` + `tenancy`): login (OIDC Google + magic links +
  optional passwords), invitations, USI club, roles, impersonation, audit.
- `members`: roster, structured diving levels, preparing level, gear defaults.
- `activities`: fosse sessions, sorties + dives, explicit DP assignments.
- `registrations`: questionnaires (magic-link and in-app), business rules,
  public recap (leak-free redesign — needed for the gear-request email).
- `palanquees`: builder (desktop + mobile), composition rules server-side,
  gear accounting (Fosse's current formula, single server implementation),
  bilan, fiche de sécurité PDF.
- `competencies`: tree/stages/validations with history, monotonic rule,
  min-validator gate, filled level PDFs from **imported** stamp positions.
- `imports`: CSV import into a session.
- `notifications`: **Dive Manager sends emails by SMTP** (product owner,
  2026-08-17) — invitations, magic links, gear-request; templates as club
  config, HTML-escaped.
- Web frontend (responsive, mobile-first) covering all of the above.
- Importer run in re-sync mode + final sync at cutover.

## Store milestone (mid-September, before full cutover)

Same codebase wrapped with Capacitor (ADR-0005). Submission 2026-09-05/08.
Store v1 minimum (product owner: "base modules are enough"): login, my
profile, my levels/competencies. If ready by submission: my dives + answer
questionnaire. Admin/DP tools stay web-only. Later app updates are cheap once
the first review passes.

## Post-cutover backlog (explicitly NOT in September)

- Level-document **visual stamp editor** (positions are imported as data;
  editing waits).
- Statistics/charts (Fosse's are silently wrong anyway).
- Sortie group builder (skill-filter view).
- Native/mobile apps, additional OIDC providers, RLS defense-in-depth,
  per-club custom permission matrix UI (seeded roles suffice at first).
- Onboarding of clubs other than USI.

## Go/no-go checklist (~2026-09-20)

- Cutover-scope modules meet the Definition of Done (incl. tenant-isolation
  and negative authz tests).
- Import reconciliation clean on real USI data (counts + spot checks + gear
  totals parity vs Fosse's summary).
- Club-admin walkthrough done: create a fosse, import a CSV, send invitations,
  fill a questionnaire via magic link, build palanquées, produce the fiche.
- Email deliverability verified from the production SMTP.
- No-go ⇒ USI keeps using Fosse; re-sync continues; new date set.

### Decommission (after cutover)
Fosse read-only for an agreed retention period, then shut down; final DB dump
archived.

## Verification at each cutover

- Row-count and spot-check reconciliation reports produced by the importer.
- Parity checks on computed values (gear accounting totals vs Fosse's summary,
  competency progress percentages) over the real dataset.
- A club-admin walkthrough of their own data before each switch.

## Risks specific to migration

- **Deadline compression** (Phase A in ~4–6 weeks): mitigated by making
  Phase A's scope import + consultation only, no operational workflows.
- **`diving_level` blob variance**: audit production data before writing the
  parser; unknown tokens must fail the import loudly, not silently.
- **Identity matching**: same email with different casing/whitespace across
  `people` — normalize and report collisions instead of merging silently.
- **Frontend availability for Phase A**: the stack decision must happen at
  workspace creation; consultation UI is deliberately tiny.
