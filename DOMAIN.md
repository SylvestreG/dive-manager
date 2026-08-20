# Domain

Consolidated domain reference for Dive Manager, derived from the Fosse
analysis (details: `docs/analysis/fosse-backend.md` and
`docs/analysis/fosse-frontend.md`). This file describes the **target** domain;
Fosse quirks are listed as inputs, not commitments.

## 1. Ubiquitous language

French diving (FFESSM) terms are kept as domain vocabulary; code identifiers
are English translations where unambiguous, French domain words where the term
is untranslatable (palanquée, sortie are acceptable as domain nouns).

| Term | Meaning |
|---|---|
| **Fosse** | Deep-pool training session (an evening at a pool) |
| **Sortie** | Multi-day dive trip (type: exploration or technique) |
| **Plongée** | One dive within a sortie (Fosse models it as a child "session") |
| **Rotation** | A wave of dives within a pool session |
| **Palanquée** | A dive team within a rotation (guides + students) |
| **DP / Directeur de plongée** | Dive director: legally responsible person for a session/dive |
| **GP / Guide de palanquée** | Dive-team guide. Member roles in a palanquée: E (encadrant), GP, P (plongeur) |
| **Encadrant** | Instructor-level member (E1–E4); may supervise/validate |
| **Niveaux** | FFESSM ladder: PESH6, PESH12 (adapted diving), N1–N5, aptitudes PE40/PA20/PA40/PE60/PA60, instructor grades E1–E4 |
| **Acquis** | A skill in the competency tree (Domaine → Module → Acquis) |
| **Étape de validation** | Validation stage for a skill (e.g. Vu en piscine → Acquis piscine → Validé en mer), ordered, with a final stage |
| **Fiche de sécurité** | Legal safety sheet PDF: palanquées with planned/realised depth & duration, DP, surface security |
| **Bloc / bouteille** | Tank (Air or Nitrox); **bloc de secours** = rescue tank |
| **Stab / détendeur** | BCD (sizes XS–XL) / regulator (a second one = octopus) |

FFESSM level hierarchy used by Fosse (numeric rank): PESH6=6, PESH12=8, N1=10,
PE40/PA20=11, N2=20, PA40/PE60/PA60=21, N3=30, E1=35, N4=40, N5=50, E2=55,
E3=60, E4=70. "Instructor" = highest completed level ∈ E1..E4.

## 2. Target domain concepts

New concepts required by multi-tenancy are marked ⊕.

- ⊕ **Club** — the tenant. Owns configuration (locations/pools, gear
  conventions, branding, email settings, enabled modules).
- ⊕ **User** — a login identity (globally unique email). Distinct from
  membership.
- ⊕ **Membership** — user ↔ club link carrying club-level roles. A user can
  belong to several clubs. Replaces Fosse's `people.group_id` + config admin
  list. Global admin is a platform-level attribute, not a club role.
- **Member profile** (per club) — name, phone, diving levels (structured, not a
  comma string), preparing level, default gear preferences.
- **Session** (fosse) and **Sortie** with its **dives** — Fosse overloads one
  `sessions` table for both; the rewrite should model Dive as its own concept
  (or a cleanly discriminated one) and fix the "list sessions = WHERE sortie_id
  IS NULL" hack.
- **Registration** (Fosse: questionnaire) — a person's participation in a
  session or sortie: gear requests, nitrox flags, transport/carpool, comments,
  submitted state. Exactly one parent (session XOR sortie) — DB-enforced.
- **Rotation / Palanquée / Palanquée member** — as in Fosse, with real
  constraints (unique numbers, enum roles, enum gas).
- **Dive director assignment** — ONE mechanism (Fosse has two: a boolean flag
  for fosse sessions, a table for sortie dives with max 4). Unify.
- **Competency tree** — Level → Domain → Module → Skill, **platform-global**
  (product decision 2026-08-19: a foundation like the FFESSM referential,
  managed by global admins; supersedes the per-club plan), with **fixed**
  validation stages and per-(user, skill) validation state **plus an
  append-only history** (Fosse keeps only the latest state while calling it
  an audit log). Validations follow the user across clubs.
- **Level document** — per-level **global** PDF template + per-skill stamp
  positions; generated filled PDF for a member.
- **Magic link / public tokens** — tokenized anonymous access for
  questionnaire submission and session recaps. Must be scoped to exactly one
  registration (Fosse bug: token resolves by person only) and must NOT leak
  other participants' tokens.
- **Email/notification** — invitation + summary emails. Fosse never sends them
  automatically (manual copy-paste cockpit); **Dive Manager sends them by
  SMTP** (product owner decision, 2026-08-17), with templates as club config.
- **CSV import** — bulk-register people into a session; per-row errors.

## 3. Business rules inventory (from Fosse, to preserve unless noted)

### Registrations
- R1: encadrant ⇒ second regulator required (`wants_2nd_reg = true`).
- R1b: non-encadrant ⇒ may not request nitrox nor a second regulator
  (nitrox **training** is the student path).
- R2: no stab requested ⇒ no stab size.
- R3: not coming from the club's departure point ⇒ no car/carpool fields.
  (Fosse applies this only on one of its three duplicated code paths — unify.)
- R4: has a car ⇒ seats ≥ 1; **the driver counts in the seat count** (UI warns
  explicitly).
- One registration per person per session (unique). Same for sorties
  (Fosse forgot the DB constraint).
- `is_encadrant` on a registration derives from the person's diving level; not
  user-editable.
- Magic-link tokens: single-use (consumed on submit), expire the day after the
  session ends. (Fosse's configured expiration_hours is dead code.)

### Sessions & sorties
- Session names auto-generated `"{lieu} - {date}"`; sortie names
  `"Sortie {Type} - {lieu} - {date}"`; dive names `"Plongée {n} - Jour {d}
  {Matin/Après-midi/…}"` with labels depending on dives/day (1–4).
- Sortie: `days_count` 1..14 × `dives_per_day` 1..4 dives auto-created, each
  with a default rotation; `end_date = start + days − 1`.
- Nitrox option only for sorties of type technique.
- Fosse DP: exactly one per pool session, must be encadrant. Sortie dives: up
  to 4 DPs (Fosse doesn't check encadrant there — treat as a bug; a DP must be
  qualified).
- Access: participants see sessions/sorties they're registered on. Members see
  only the next upcoming dive of a sortie unless they are DP (then all dives).
  Fosse's rule "any past encadrant gets DP tools on the next upcoming sortie"
  is **confirmed accidental** (product owner, 2026-08-17) and is not
  reproduced: DP tools come from explicit assignment only.

### Palanquées & rotations
- Palanquée composition: max 2 guides (GP zone) + max 4 students. A person
  appears at most once per rotation.
- Cross-rotation rules are **location-dependent** (club configuration in the
  rewrite, hardcoded strings in Fosse):
  - Club's own pool (Coubertin): a student joins one rotation, or two if
    `allow_dual_rotation` (student-only flag).
  - Partner pools (Montluçon, Le Puy-en-Velay): evening has Plongée 1 and
    Plongée 2; students are expected on both; rotations carry a
    `plongee_number`.
  - Encadrants: unlimited across rotations, always re-selectable.
- Gas assignment inferred: Nitrox iff (wants_nitrox ∨ nitrox_training), else Air.
- Rotation ordering: by plongée number then rotation number (Fosse's code and
  docs disagree on None-first — decide explicitly).
- Planned params: duration ≤ 120 min, depth ≤ 60 m; realised params (bilan):
  duration ≤ 180 min, depth ≤ 120 m; a palanquée is "complete" when both
  realised values are set.

### Gear accounting (club pool only — currently a club convention)
- Tanks: one per participant + 1 rescue tank (Air).
- Nitrox tanks = participants with (wants_nitrox ∨ nitrox_training); air = rest.
- "Mode 2 rotations" (optimization): students share tanks across their two
  rotations → `ceil(students_air + 1 (rescue) / 2)` air + `ceil(students_nitrox
  / 2)` nitrox for students; encadrant tanks not optimized.
  ⚠ Product owner (2026-08-17): the exact formula is "probably to fix, revisit
  later" — implement Fosse's current formula as the single server-side
  implementation, isolated so it can be corrected without touching callers.
- Regulators = requesters + 1; stabs = requesters + 1 (rescue size bucket).
- ⚠ Fosse computes this in the backend summary AND recomputes divergently in
  4 frontend locations; the backend must be the single source of truth.

### Competencies (as implemented, 2026-08-19/20 — supersedes Fosse's rules)
- Tree: Level → Domain → Module → Skill, platform-global.
- **Fixed stages** (no table, no editor): `vu → à perfectionner → validé`.
  N1 and below: any environment; above N1 the final validation is
  necessarily **at sea** (label "Validé en mer", same stage).
- **Required validator grade is a function of the level** (not per skill):
  E1 validates < N2, E2 < N3, E3 < N4, E4 beyond — compared to the DERIVED
  certified teaching grade (ADR-0007; the stagiaire MF1 status grants
  nothing). Validator and diver must share an active club membership.
- Progression is monotonic: a stage can only advance (or re-stamp the same
  stage); going backwards or deleting requires a REAL global admin. Every
  write appends an immutable, denormalized history event.
- No self-validation; validation dates cannot be in the future; competency
  writes are refused while impersonating.
- Only final-stage validations count as "validated" in progress stats
  (weighted percentage, single server-side formula) and appear on the
  generated level PDF (flattened text, not removable annotations).
- Preparing level: a member prepares at most one level (users module rules).
- Level cascade: validating a level implies the lower ones — covered by the
  users module (`ffessm::covered_by` certification tree).

### Impersonation
- Global/club admin can impersonate a member; UI must make it unmistakable;
  permissions are the target's; sensitive actions (validation rollback,
  deletes, logs) require the real, non-impersonating admin.

## 4. Fosse defects that shape the rewrite (do not reproduce)

Security: fail-open ACL (no data ⇒ all permissions); authenticated-but-not-
authorized mutation routes (incl. rewriting group permissions); public recap
leaking every participant's magic link and email; permissive CORS; `is_admin`
frozen in a 24 h JWT; no login rate limiting; unescaped HTML email templates.

Correctness: magic-link token resolving a questionnaire **by person only**
(reads/overwrites the wrong registration for multi-session members); business
rules triplicated and drifted; two DP mechanisms; two competency models;
`diving_level` comma-string parsed five different ways with three divergent
frontend rank tables; Some-only updates that can never clear a field; dead
config (`magic_link.expiration_hours`); no transactions; N+1 queries; no
validation history.

Product debt: club branding, geography (Issoire), pool names, rescue-tank
convention, and email templates hardcoded across backend, frontend and HTML
template — all become club configuration.

## 5. Preserve / redesign / drop (initial position — to confirm)

**Preserve (proven value):** pool-session + sortie workflows; magic-link
questionnaires; palanquée builder UX (drag & drop + mobile tap-to-assign,
bottle budget); fiche de sécurité PDF; competency tree with custom stages and
min-validator levels; level-document stamp editor; bilan (realised params)
workflow; skill-based group builder for sorties; impersonation with strong
visual framing; CSV import; public session recap (without the token leak);
profile-completeness nudge; auto-generated names; dark/light theme.

**Redesign:** everything tenant-related; roles/permissions (explicit grants,
no fail-open); session/dive modeling; single DP mechanism; structured diving
levels; registration business rules in one backend place; gear accounting
server-side; locations/pools as club config with per-pool behavior flags;
validation history; email sending strategy; token scoping.

**Drop (dead or superseded in Fosse):** legacy flat competencies; the no-op
`copy_attendees`; unreachable pages (SessionEmailsPage, ImportPage as a
standalone orphan, competences-legacy); `people.default_is_encadrant`;
sortie summary tokens (never consumed); the "fabricate a 1-row CSV to add a
participant" pattern (a real endpoint instead).
