# Fosse — Functional Inventory

Per-feature inventory derived from `fosse-backend.md` and `fosse-frontend.md`
(2026-08-17). **Confirmed** = read in the code. **Inferred** = deduced from
code/UX but not stated anywhere. **Unknown** = requires the product owner or
production data.

Verdicts: **migrate** (keep as is, re-implemented cleanly), **redesign**
(keep the need, change the solution), **remove**.

Global unknowns (apply to everything): actual production usage frequency of
each feature; volume of data; which club conventions are negotiable.

---

## 1. Authentication (Google OAuth, One Tap, passwords, temp passwords)

- **Purpose**: let members and admins log in.
- **Users**: everyone.
- **Confirmed rules**: Google login allowed only for config-admins or existing people rows; Gmail addresses are forced into Google-only login by the frontend; temp password = 6 chars, 24 h, Argon2-hashed, enumeration-safe response; password min 8 chars; JWT 24 h carrying `is_admin`.
- **Data**: `people` (password fields), config `admin.emails`.
- **Implementation**: `services/auth.rs`, `handlers/auth.rs`, `LoginAdmin.tsx`.
- **Edge cases**: case-sensitive email matching at login vs case-insensitive elsewhere; `must_change_password` never enforced server-side.
- **Verdict**: **redesign** — ADR-0004 (invitation-only, OIDC generic, login magic links, optional passwords, revocable sessions).
- **Inferred**: the Gmail-forcing heuristic exists to push people toward OAuth; nothing documents it.

## 2. Authorization (groups & permissions)

- **Purpose**: differentiate admin / encadrant / élève capabilities.
- **Users**: admins configure; everyone is subject to it.
- **Confirmed rules**: 3 seeded groups; 22-permission catalogue; permission matrix editable via `PUT /groups/:id`; fail-open when person unknown or ungrouped; most mutation routes check nothing beyond authentication; `is_admin` comes only from config emails.
- **Data**: `groups`, `group_permissions`, `people.group_id`.
- **Edge cases**: DB seed and Rust defaults disagree (nobody can validate competencies out of the box).
- **Verdict**: **redesign** — fail-closed per-club roles (foundation design); keep the *UX* of an editable permission matrix.
- **Unknown**: whether USI ever edited the permission matrix in production.

## 3. People management

- **Purpose**: club roster CRUD, diving levels, gear defaults.
- **Users**: admins (UsersPage); members read/update a subset (profile).
- **Confirmed rules**: email globally unique; `diving_level` comma-string incl. `preparing_*`; level cascade + next-preparable-level constraints implemented **frontend-only**; `is_instructor` derived (highest complete level E1–E4); gear defaults prefill registrations.
- **Data**: `people`.
- **Edge cases**: three divergent level-ranking tables in the frontend; import-created people get different gear defaults than manually created ones; optional fields can never be cleared via update.
- **Verdict**: **redesign** — structured levels in DB, cascade/preparing rules in the backend, profiles-without-accounts kept.

## 4. Pool sessions (fosses)

- **Purpose**: organize a pool evening: participants, DP, rotations, recap.
- **Users**: admins create/manage; DP manages participants; members view/register.
- **Confirmed rules**: name auto-generated `{lieu} - {date}`, immutable after creation (only `optimization_mode` is updatable); locations hardcoded (Coubertin/Montluçon/Le Puy-en-Velay); one DP max, must be encadrant; summary token generated at creation.
- **Data**: `sessions` (where `sortie_id IS NULL`).
- **Edge cases**: blank location silently means Coubertin (legacy data).
- **Verdict**: **migrate** the workflow, **redesign** the model (locations = club config with behavior flags; sessions editable; separate concept from sortie dives).

## 5. Sorties (dive trips) & dives

- **Purpose**: multi-day trips; auto-generate dives; manage DPs and participants per trip.
- **Users**: admins create; DPs manage; encadrants/members participate.
- **Confirmed rules**: type exploration|technique; nitrox option only for technique; days 1..14 × dives/day 1..4 children auto-created with time-of-day labels and one default rotation each; ≤4 DPs per dive, no encadrant check; DP-tool access = was DP anywhere ∨ (any-encadrant-ever ⇒ next upcoming sortie); members see only the first upcoming dive unless DP.
- **Data**: `sorties`, child `sessions`, `dive_directors`.
- **Edge cases**: `copy_attendees` is a no-op stub; sortie summary tokens generated but never consumable; sortie magic links never delivered (`email_jobs.sortie_id` never written).
- **Verdict**: **migrate** the workflow; **redesign** dive modeling (own concept), DP assignment (single mechanism, qualification required, explicit grants instead of the "next sortie" implicit rule).
- **Unknown**: whether the "any encadrant gets the next sortie" behavior is a wanted feature or an accident.

## 6. Registrations / questionnaires + magic links

- **Purpose**: collect participation, gear, nitrox, transport per person per session/sortie.
- **Users**: members (magic link or logged in), admins, DPs.
- **Confirmed rules**: R1 encadrant ⇒ 2nd regulator; R1b non-encadrant ⇒ no nitrox/2nd reg request; R2 no stab ⇒ no size; R3 not-from-Issoire ⇒ no car (submit path only — drifted); R4 car ⇒ seats ≥ 1, driver counts; unique (session, person); token single-use, expires D+1 after session; `is_encadrant` non-editable; submit-time consumed flag.
- **Data**: `questionnaires`, `email_jobs` (token).
- **Edge cases (confirmed bugs)**: token resolves by **person only** → wrong questionnaire for multi-session members; public submit silently drops `comes_from_issoire` and nitrox-formation fields; submit on non-pre-created questionnaire → 404; no unique (sortie, person) in DB.
- **Verdict**: **migrate** the feature; **redesign** token scoping, rule unification, transport model (club-configurable departure points instead of `comes_from_issoire`).

## 7. Public session recap (summary token)

- **Purpose**: shareable no-login recap: counts, gear, participant lists (used in the gear-request email to the club's equipment manager).
- **Confirmed rules**: valid until D+1 23:59 after the session; **exposes every participant's email and magic link** (security hole).
- **Verdict**: **migrate** the feature (real value), **redesign** the payload (no tokens, no emails; aggregate + names only) and add explicit expiry semantics.
- **Inferred**: the audience is the equipment manager and carpool organizers.

## 8. Palanquées & rotations builder

- **Purpose**: compose dive teams per rotation; the operational core.
- **Users**: admin and DP edit; registered members view.
- **Confirmed rules**: ≤2 GP + ≤4 students per palanquée; one appearance per rotation; cross-rotation limits depend on location (partner pools: both plongées expected; Coubertin: 1 rotation, 2 if `allow_dual_rotation`, students only; encadrants unlimited); gas inferred (Nitrox iff wants_nitrox ∨ nitrox_training); planned params ≤120 min/≤60 m; rotation ordering None→1→2 (doc says otherwise); participants sourced from sortie when the session is a dive.
- **Data**: `rotations`, `palanquees`, `palanquee_members`.
- **Edge cases**: composition limits enforced **client-side** (server: none found — mutations lack authorization entirely); unique numbers not DB-enforced; `role`/`gas_type` free strings.
- **Verdict**: **migrate** the UX (drag & drop + mobile tap-to-assign is genuinely good); **redesign** enforcement (all rules server-side, real constraints, authz on mutations).

## 9. Gear/tank accounting ("mode 2 rotations")

- **Purpose**: compute tanks/regulators/stabs to prepare, incl. rescue gear and the tank-sharing optimization.
- **Confirmed rules**: +1 rescue tank (Air) / +1 regulator / +1 stab "Secours"; nitrox tanks = wants_nitrox ∨ nitrox_training; optimization: students share tanks over 2 rotations (`ceil/2` per gas); partner pools: no gear accounting at all.
- **Edge cases**: backend computes it in the summary, frontend ignores that and recomputes in 4 divergent places.
- **Verdict**: **migrate** the need; **redesign**: one server-side implementation; conventions (rescue gear, optimization) become club config.
- **Unknown**: whether the exact `ceil((students_air + 1)/2)` formula (rescue tank inside the student optimization) is intended or accidental — validate with the club before freezing it.

## 10. Fiche de sécurité (safety sheet PDF)

- **Purpose**: legal document per session/dive: palanquées, roles, gas, planned/realised params, DP, surface security.
- **Users**: admin, DP.
- **Confirmed rules**: header fields overridable at generation (club defaults "USI Plongée"); partner pools get a per-plongée scope; realised params entered via the "bilan" workflow (≤180 min/≤120 m, completeness indicator, "download anyway?" guard); DP resolution prefers the fosse flag then dive_directors; E/GP/P legend; `LASTNAME F.` name format.
- **Verdict**: **migrate** (core value); **redesign** the PDF implementation (hand-rolled content streams, two divergent accent tables → a proper PDF library or HTML-to-PDF, decided at module design).

## 11. Competency management (tree, stages, validations)

- **Purpose**: track training progression per member toward a prepared level.
- **Users**: admins (structure), encadrants (validate), members (view own).
- **Confirmed rules**: Level → Domain → Module → Skill; skill has `min_validator_level`; validator must hold ≥ that level; stage progression monotonic except real admin; upsert per (person, skill) — no history; only `is_final` stages count as validated (stats + PDF); stages CRUD-able with color/icon/order; 5 seeded stages; validation logs page (real admin).
- **Data**: `competency_*`, `validation_stages`, `skill_validations`.
- **Edge cases**: two definitions of "can validate" (permission vs E1–E4 heuristic); `update` ignores impersonation for attribution; legacy flat `competencies` model still routed.
- **Verdict**: **migrate** structure & rules; **redesign** storage (append-only history + current state), per-club trees; **remove** the legacy flat model.
- **Unknown**: whether validation history loss has ever hurt (motivates but doesn't prove the history requirement).

## 12. Level documents (PDF stamp editor + filled PDFs)

- **Purpose**: upload official level PDF; place per-skill stamp boxes visually; generate a member's filled document.
- **Users**: admins place; members download their own; encadrants/admins download any.
- **Confirmed rules**: one template per level; positions unique (skill, level); only final-stage validations printed (date + validator + grade).
- **Edge cases**: PDFs stored as bytea in Postgres; WinAnsi ~35-char accent table (others become `?`); any authenticated user can upload/delete templates and generate anyone's PDF (no authz).
- **Verdict**: **migrate** (distinctive, valuable feature); **redesign** authz + storage decision (DB blob vs disk — decide at module design; blobs are simple and backup-friendly at club scale).

## 13. Sortie group builder (skill-based filtering)

- **Purpose**: constitute homogeneous training groups for a sortie by filtering students on validated skills/stages.
- **Users**: admins, encadrants.
- **Confirmed rules**: filter by prepared level, name, stage, and AND-set of skills validated at any stage.
- **Verdict**: **migrate** as part of `competencies` × `activities` (a view, not a module).

## 14. CSV import

- **Purpose**: bulk-register people into a session; creates people, registrations, magic links, email jobs.
- **Users**: admins.
- **Confirmed rules**: header `first_name,last_name,email,phone?`; find-or-create by email; per-row error report (`import_jobs`); non-transactional by design (partial success is normal).
- **Edge cases**: naive email validation; UTF-8 only, no BOM; the frontend "add one participant" fabricates a 1-row CSV in memory; import-created people get all-false gear defaults.
- **Verdict**: **migrate**; **redesign** the single-participant path (real endpoint) and person-creation defaults consistency.

## 15. Emails

- **Purpose**: invite participants (magic links), request gear, share recaps.
- **Users**: admins.
- **Confirmed rules**: the system **sends nothing** except the temp-password email; invitation bodies are generated and stored (`email_jobs`) then manually copy-pasted; the current UI (EmailsPage) is a pure frontend copy-paste cockpit with hardcoded club logistics text and a personal phone number; subject/body conventions confirmed in code.
- **Edge cases**: template interpolated without HTML escaping; `magic_link.expiration_hours` dead; legacy queue page unreachable.
- **Verdict**: **redesign** — decision pending (send vs assist, see open questions); templates become club config either way.
- **Unknown**: why sending was never automated (deliverability fear? Gmail workflow preference?). Ask.

## 16. Impersonation

- **Purpose**: admin acts as a member to see what they see / fix their data.
- **Confirmed rules**: admin-only; permissions become the target's; red ring + banner; "real admin" required for destructive actions; 401s swallowed client-side while impersonating.
- **Verdict**: **migrate** (with audit logging added; foundation design covers it).

## 17. Dashboard, profile, "my dives"

- **Purpose**: personal home (stats, upcoming events, profile-completeness nudge), gear preferences, past palanquées with teammates, instructor shortcut to a student's validation page.
- **Confirmed rules**: gear/transport editable in-app only for the club's own pool sessions; "my sorties" shows first upcoming dive only unless DP.
- **Edge cases**: N+1 waterfalls make these pages the slowest in the app.
- **Verdict**: **migrate** the UX; **redesign** with purpose-built aggregate endpoints.

## 18. Statistics (competency charts)

- **Purpose**: progression charts for admins/encadrants.
- **Confirmed rules**: computed client-side over capped data (`slice(-10)` sessions, 20 students) — **silently wrong aggregates**.
- **Verdict**: **redesign later** (low priority; correct server-side aggregates when the need is confirmed).

## 19. Confirmed-dead features (remove)

- Legacy flat competencies model (superseded, still routed).
- `copy_attendees` endpoint (no-op stub).
- Orphan pages: SessionEmailsPage (legacy email queue), standalone ImportPage route, competences-legacy.
- Sortie summary tokens (no consumer), `people.default_is_encadrant` (never read), unused API surface listed in the analyses.
