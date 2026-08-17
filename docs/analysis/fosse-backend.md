# Fosse Backend — Functional Analysis

> Analysis of the reference application's backend (`../fosse/backend`),
> performed 2026-08-17. Rust, Axum 0.7 + SeaORM 1 + PostgreSQL, ~15.6 kLOC.
> Feeds `DOMAIN.md` and `MODULES.md`.
> Note: the README claims Actix-web; the code uses Axum.

---

## 1. Domain model

Entities in `fosse/backend/src/entities/`, created by 33 migrations (all with
the same fake `m20240101_` date prefix). All PKs are application-generated
UUIDv4. All timestamps naive UTC, written by handlers (no DB triggers). No
soft deletes.

### 1.1 `sessions` — a pool evening ("fosse") **or** one dive of a sortie

| Field | Notes |
|---|---|
| `id` Uuid PK | |
| `name` | e.g. "Plongée 3 - Jour 2 Après-midi" when auto-generated |
| `start_date` Date, `end_date` Option<Date> | |
| `location` Option<String> | **club-specific**: matched against `"Coubertin"`, `"Montluçon"`, `"Le Puy-en-Velay"` |
| `summary_token` Option<Uuid> unique | public recap link token |
| `optimization_mode` bool | "mode 2 rotations"; only field togglable via PUT |
| `sortie_id` Option<Uuid> → sorties CASCADE | non-null ⇒ this "session" is a dive inside a sortie |
| `dive_number` Option<i32> | ordering inside a sortie |

**Dual meaning overload**: `sessions` is both a standalone pool session and a
dive child of a sortie. `list_sessions` filters `sortie_id IS NULL`.

### 1.2 `sorties` — sea/dive trip

`name`, `location` (NOT NULL), `sortie_type` (`"exploration"|"technique"`, no
DB CHECK), `days_count`, `dives_per_day`, `nitrox_compatible`, `start_date`,
`end_date` (computed = start + days_count − 1), `description`, `summary_token`
unique (**generated but no public endpoint consumes it — dead feature**).

### 1.3 `people` — every human

| Field | Notes |
|---|---|
| `email` | **globally unique**, case-sensitive at DB level; code inconsistently lowercases |
| `default_is_encadrant` | effectively dead: always written `false`, overwritten in responses by `is_instructor` derived from `diving_level` |
| `default_wants_regulator` (true), `default_wants_nitrox`, `default_wants_2nd_reg`, `default_wants_stab` (true), `default_stab_size` | gear preferences |
| `diving_level` Option<String> | **comma-separated multi-value blob**, e.g. `"N1,N2,N3,E2"` + pseudo-tokens `preparing_N4` |
| `group_id` Option<Uuid> → groups | ACL group |
| `password_hash`, `temp_password`, `temp_password_expires_at`, `must_change_password` | Argon2 auth |

Migration 010 added `default_comes_from_issoire`/`default_has_car`/
`default_car_seats`; 011/013 dropped them ("session-specific").

### 1.4 `questionnaires` — a registration/participation record

| Field | Notes |
|---|---|
| `session_id` Option, `sortie_id` Option | exactly one should be set; **no CHECK constraint** |
| `person_id` → people CASCADE | |
| `is_encadrant` | set from diving level at import; also the "can be DP" gate |
| gear: `wants_regulator`, `wants_nitrox`, `wants_2nd_reg`, `wants_stab`, `stab_size` | |
| `nitrox_training`, `nitrox_base_formation`, `nitrox_confirmed_formation` | |
| `is_directeur_plongee` | the single fosse-DP flag |
| `allow_dual_rotation` | club-specific: "fosse Coubertin: student may take 2 rotations" |
| `comes_from_issoire` bool **default TRUE** | **hardcoded club geography** |
| `has_car`, `car_seats`, `comments`, `submitted_at` | `submitted_at IS NULL` ⇒ not answered |

Unique `(session_id, person_id)`. **No unique on `(sortie_id, person_id)`** —
only an application-level check.

### 1.5 `email_jobs` — magic-link + outbound email queue

`session_id` Option, `sortie_id` Option (**never written**), `person_id`,
`questionnaire_token` Uuid unique, `status` (`generated|sent|failed`, free
string), `retry_count`, `sent_at`, `expires_at`, `consumed`, `email_subject`,
`email_body` (fully rendered HTML stored in DB).

### 1.6 `import_jobs`

`session_id`, `filename`, `status`, `total_rows`, `success_count`,
`error_count`, `errors` JSON `{row, message}[]`.

### 1.7 Rotations / palanquées

- **`rotations`**: `session_id` CASCADE, `number`, `plongee_number` Option (1|2 at partner pools; `None` = club fosse). No unique `(session_id, number)`.
- **`palanquees`**: `rotation_id` CASCADE, `number`, `call_sign`, planned params (departure time, duration min, depth m), actual params. No unique `(rotation_id, number)`.
- **`palanquee_members`**: `palanquee_id`, `questionnaire_id`, `role` (`"E"|"GP"|"P"`, free string), `gas_type` (default `"Air"`, unvalidated). Unique `(palanquee_id, questionnaire_id)`.

### 1.8 `dive_directors`

`session_id`, `questionnaire_id`, unique pair. Parallel/duplicate mechanism to
`questionnaires.is_directeur_plongee` (1 DP per fosse vs ≤4 DPs per sortie dive).

### 1.9 Competency hierarchy

- **`validation_stages`**: `code` unique, `name`, `color`, `icon`, `sort_order`, `is_final`. Seeded: `VU_PISCINE`, `A_PERFECTIONNER`, `ACQUIS_PISCINE`, `A_RETRAVAILLER_MER`, `VALIDE_MER` (final).
- **`competency_domains`**: `diving_level` (e.g. "N2"), `name` (COMMUNES / PE40 / PA20…), `sort_order`.
- **`competency_modules`**: `domain_id`, `name`, `sort_order`.
- **`competency_skills`**: `module_id`, `name`, `description`, `sort_order`, `min_validator_level` (default `"E2"`).
- **`skill_validations`**: `person_id`, `skill_id`, `stage_id`, `validated_at`, `validated_by_id`, `notes`. Unique `(person_id, skill_id)` ⇒ **only latest state, no history**.
- **`competencies`**: old flat model, superseded but still routed ("legacy").

### 1.10 Level documents / PDF overlay

- **`level_documents`**: `level` unique, `file_name`, `file_data` (**PDF blob in Postgres**), `page_count`.
- **`skill_document_positions`**: `skill_id` CASCADE, `level`, `page`, `x`, `y`, `width`, `height`, `font_size` (default 8.0). Unique `(skill_id, level)`.

### 1.11 ACL

- **`groups`**: `name` unique, `group_type` (`admin|encadrant|eleve`), seeded with exactly 3 rows; no create/delete endpoint.
- **`group_permissions`**: `group_id`, `permission` string, unique pair. Seeded per group_type.

---

## 2. Business rules by feature area

### 2.1 Sessions (`handlers/sessions.rs`)

- Creation always generates a `summary_token`; `optimization_mode = false`.
- Name/date/location **immutable after creation**; update only accepts `optimization_mode`.
- `get_session` access: `SessionsView` ∨ real admin ∨ registered participant (via session or parent sortie).
- **Session summary** (hardcoded club gear accounting): `total_bottles = participants + 1` (bloc de secours), `nitrox_bottles = count(wants_nitrox || nitrox_training)`, `air = total - nitrox` (rescue tank is Air), `regulators = count(wants_regulator) + 1`, `stabs = count(wants_stab) + 1` + synthetic size bucket `"Secours"`; also `from_issoire_count`, `vehicles_count`, `total_car_seats`, per-participant magic link.
- **Public summary by token**: valid until `end_date (or start_date) 23:59 + 1 day`, then 404. **Fully public — leaks names, emails and magic links** (see §6.18).

### 2.2 Sorties (`handlers/sorties.rs`)

- Create/update/delete **admin-only via `claims.is_admin`** (config email list, not ACL). `days_count` 1..14, `dives_per_day` 1..4.
- Auto-generates `days × dives` child sessions named `"Plongée {n} - Jour {d} {label}"`, labels by dives_per_day (1→[""], 2→[Matin, Après-midi], 3→[+Midi], 4→[Matin, Fin de matinée, Début d'après-midi, Après-midi]); each child gets sortie's location, no summary token, and **one default rotation**.
- `list_sorties`: **no access filtering** — any authenticated user sees all.
- `copy_attendees` is a **no-op stub** returning counts without writing.

### 2.3 Questionnaires (`services/questionnaire.rs`)

Three creation paths: magic link (public), CSV import (pre-created,
unsubmitted), self/DP registration (`POST /questionnaires/register`, creates
the person if unknown).

**Business rules** (3 near-identical drifted copies):
- R1: `is_encadrant` ⇒ `wants_2nd_reg = true`.
- R1b: not encadrant ⇒ `wants_nitrox = false` and `wants_2nd_reg = false`.
- R2: `!wants_stab` ⇒ `stab_size = None`.
- R3: `!comes_from_issoire` ⇒ `has_car = false`, `car_seats = None` (**submit variant only** — update/create dropped it).
- R4: `has_car` ⇒ `car_seats >= 1`.

Token flow: expired ⇒ 401 `InvalidToken`; consumed ⇒ 410 `TokenConsumed`;
successful submit marks consumed + `submitted_at = now`. `is_encadrant` not
editable via update.

**Fosse DP**: at most one per session (clear-then-set); questionnaire must be
`is_encadrant` and belong to that session. `allow_dual_rotation` rejected for
encadrants.

### 2.4 Palanquées & rotations (`handlers/palanquees.rs`)

Two location predicates drive everything:
`student_stays_in_pool_until_both_plongees(loc)` = Le Puy-en-Velay | Montluçon;
`is_coubertin_club_fosse(loc)` = None | "" | "Coubertin" (**blank location
silently means Coubertin** — legacy-data compat).

- Rotation `number` auto-increments per session; `plongee_number` accepted only as 1|2, else silently dropped.
- **Rotation ordering** (`rotation_order.rs`): `None` first, then 1, then 2, ties by number. Doc comment says the opposite — contradiction.
- **Member addition constraints**: no duplicate within a rotation; non-encadrant students at partner pools have no cross-rotation limit (they dive plongée 1 and 2); at Coubertin a student already in ≥1 rotation is rejected unless `allow_dual_rotation` and currently < 2 rotations; encadrants unlimited.
- Default gas: `Nitrox` iff `wants_nitrox || nitrox_training`.
- **Unassigned pool**: encadrants always selectable; partner pools until covered on both plongées; Coubertin + dual until in 2 rotations; else removed once assigned. Sorted encadrants-first.
- Participants sourced from the **sortie** questionnaires if `session.sortie_id` set, else session's own.
- Member display order: E < GP < P, then last name.
- Read access checked; **all mutation endpoints have no per-resource authorization — only "is authenticated"**.

### 2.5 Dive directors (sorties)

Max 4 per dive (hardcoded); no duplicate; questionnaire must belong to the
sortie; **no `is_encadrant` check** (unlike fosse DP). Add/remove: real admin ∨
sortie director-tool access. PDF generator prefers the fosse flag, falls back
to `dive_directors`.

### 2.6 Competencies & skill validations (`handlers/competency_hierarchy.rs`)

- Create validation requires `CompetenciesValidate`; validator resolved by (impersonated) email.
- **Level gate**: at least one parsed level of the validator ≥ `skill.min_validator_level` in the hierarchy. No diving_level ⇒ 403.
- **Upsert**: `(person, skill)` unique — update, not append.
- **No going backwards**: lower `sort_order` requires real admin (admin and not impersonating).
- Delete and logs endpoint: real admin only.
- Progress stats: `validated` counts only `is_final` stages.

**Diving level ladder** (`models/diving_level.rs`): PESH6=6, PESH12=8, N1=10,
PE40/PA20=11, N2=20, PA40/PE60/PA60=21, N3=30, E1=35, N4=40, N5=50, E2=55,
E3=60, E4=70. "Instructor" = highest complete level ∈ E1..E4.
`auth.rs` uses a **different** rule for `can_validate_competencies`: any token
among E1..E4.

### 2.7 Level documents & filled PDFs

One template per level (bytea blob); admin places per-skill rectangles; the
filled PDF renders **only final-stage validations** as hand-built FreeText
annotations, with a hand-rolled UTF-8→WinAnsi table (~35 chars, rest becomes `?`).

### 2.8 CSV import (`services/import.rs`)

Multipart `session_id` + `file`; UTF-8 only; header
`first_name,last_name,email,phone?`. Per row: naive email check (`contains('@')`),
find-or-create person (**created with all gear defaults false — inconsistent
with `create_person`** which defaults regulator/stab true), create questionnaire
prefilled from person defaults with `is_encadrant` derived from diving level,
generate magic-link email job. Per-row errors collected; **not transactional**.
`CsvImportRow::validate()` exists but is never called.

### 2.9 Emails (`services/email.rs`)

- Template `email_template.html` loaded **from CWD on every render**; `String::replace` placeholders, **no HTML escaping**.
- Magic link `{base_url}/q/{token}`; **expiry ignores `magic_link.expiration_hours`** (dead config): always session end date + 1 day 23:59.
- Jobs created `status = "generated"`; **nothing sends them** — manual copy-paste + "mark as sent" workflow. The only SMTP email actually sent is the temp-password mail, HTML inlined in Rust, hardcoding "USI - Commission Technique".
- `email_jobs.sortie_id` is read but never written ⇒ sortie magic links always None.

### 2.10 Fiche de sécurité (`services/fiche_securite.rs`)

Query overrides `date, club, site, position, securite_surface, observations,
plongee_number`. A4 landscape built from scratch with raw content streams
(`writeln!` + `.unwrap()`), 7 columns, alternating shading, multi-page,
legend E/GP/P. Second, divergent accent-escape table. `effectif_unique` =
distinct questionnaires across included palanquées.

---

## 3. Auth & permissions

### 3.1 Authentication

| Method | Notes |
|---|---|
| Google OAuth code flow | token + userinfo round-trips |
| Google One Tap ID token | verified via `tokeninfo` HTTP endpoint; checks `aud` only — **no `iss`/`email_verified`/local signature validation** |
| Temp password by email | 6 chars of `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`, Argon2, 24 h, generic response (enumeration-safe) |
| Email + password | temp password tried first; cleared on success |
| Change password | min length 8 only |

- Google login allowed only if email ∈ config admin list ∨ a `people` row exists (case-sensitive match).
- **JWT** HS256: `sub, email, name, exp, is_admin, impersonating?`. `required_spec_claims` cleared ("backward compat"). No refresh, no revocation, no jti. **`is_admin` comes solely from config `admin.emails`** (case-sensitive) and is trusted for the full TTL.
- **Impersonation**: admin-only; token keeps admin's identity + `impersonating{...}`; permissions resolved from impersonated email; sensitive actions require "real admin" (`is_admin && !impersonating`).
- `can_validate_competencies` in the auth response is advisory (frontend); backend gate is the permission.

### 3.2 ACL

Middleware authenticates and injects `AuthUser { claims, permissions }`.
`get_permissions_for_email` is **fail-open**: unknown email ⇒
`Permission::all()`; known person with `group_id IS NULL` ⇒ `Permission::all()`
(comment claims an admin-list check that doesn't exist).

22 permissions: sessions_view/create/edit/delete, questionnaires_view/edit/
delete, users_view/create/edit/delete, competencies_view/create/edit/delete/
validate, emails_view/send, import_csv, groups_view/edit, summaries_view.

**DB seed vs Rust `default_permissions()` disagree** — the seed gives neither
admin nor encadrant `competencies_validate`, so out of the box nobody in the DB
can validate competencies unless permissions are edited or the fail-open path
applies. Groups: only `PUT /groups/:id` (replace permission list) exists.

### 3.3 What is actually enforced

Router applies authentication only; per-handler authorization is very uneven:
- Real permission checks only in competency handlers + a few read gates.
- `claims.is_admin` checks: sortie CUD, impersonation, validation delete/logs.
- **No check at all** (any logged-in user): session CUD, all people CRUD, group reads **and `update_group_permissions`**, legacy competencies CRUD, all level-documents endpoints, all rotation/palanquée/member mutations, fiche de sécurité download, `questionnaires-detail` (leaks magic links), emails endpoints, CSV import, generate-links, list_sorties.

### 3.4 Sortie access model (`sortie_access.rs`)

Director-accessible set = sorties where the person was DP (flag or table) **plus
the next upcoming sortie if the person is `is_encadrant` on any questionnaire
ever**. That last rule over-grants: any past encadrant gets DP tools (add/remove
DPs, edit/delete others' questionnaires, register people) on the next sortie.

---

## 4. API surface

Prefix `/api/v1`. Public: auth config/callbacks/login/request-password,
`questionnaires/by-token/:token`, `questionnaires/submit`,
`sessions/summary/:token` (public recap). Authenticated: change-password,
impersonate/stop, questionnaires/register.

Sessions: POST/GET `/sessions`, GET/PUT/DELETE `/sessions/:id`, GET
`/sessions/:id/summary`, POST `/sessions/:id/generate-links`, POST
`/sessions/:id/directeur-plongee`. Questionnaires: GET `/questionnaires`,
GET `/questionnaires-detail`, PUT/PATCH/DELETE `/questionnaires/:id`.

Sorties: POST/GET `/sorties`, GET/PUT/DELETE `/sorties/:id`, GET
`/sorties/:id/questionnaires`, POST `/sorties/:id/copy-attendees` (no-op), GET
`/me/sorties-director-access`, GET/POST `/sessions/:sid/dive-directors`,
DELETE `/sessions/:sid/dive-directors/:id`.

Palanquées: GET `/sessions/:sid/palanquees`, GET `/sessions/:sid/fiche-securite`,
POST `/rotations`, GET `/sessions/:sid/rotations`, DELETE `/rotations/:id`,
POST `/palanquees`, PUT/DELETE `/palanquees/:id`, POST
`/palanquees/:pid/members`, PUT/DELETE `/palanquee-members/:id`.

People/groups/import/emails: POST/GET `/people`, GET/PUT/DELETE `/people/:id`,
GET `/permissions`, GET `/groups`, GET/PUT `/groups/:id`, POST `/import`, GET
`/import/:id`, GET `/emails/pending`, GET `/emails/session/:id`, POST
`/emails/:id/sent`.

Competencies: legacy `/competencies*` CRUD; hierarchical
`/validation-stages*`, `/competency-domains*`, `/competency-modules*`,
`/competency-skills*`, `/skill-validations*` (+ `/logs`), `/my-competencies`,
`/person-competencies/:person_id`.

Level documents: GET `/level-documents`, GET/DELETE `/level-documents/:level`,
POST `.../upload`, GET `.../download`, GET `.../page/:page`, GET/POST/PUT
`.../positions`, DELETE `.../positions/:skill_id`, GET
`.../generate/:person_id`.

Static SPA from `../frontend/dist` (CWD-relative), SPA fallback;
`CorsLayer::permissive()` in both modes.

---

## 5. Single-club assumptions

1. `admin.emails` config array is the sole source of admin identity.
2. `people.email` globally unique — two clubs could never share a member.
3. **No `club_id` anywhere**; sessions, sorties, groups, competency trees, level documents, validation stages are all global.
4. `questionnaires.comes_from_issoire` hardcodes the club's home town, default `true`; drives carpool rules and summary counts.
5. Location string matching: `["Le Puy-en-Velay","Montluçon"]` = partner pools; `None|""|"Coubertin"` = club pool. Mirrored in frontend `fosseLocations.ts` + hardcoded `<select>`.
6. `allow_dual_rotation` is Coubertin-only by construction.
7. Branding "USI - Commission Technique" hardcoded in Rust (temp-password email) and config example (OVH SMTP).
8. `email_template.html` hardcodes USI branding and "Départ d'Issoire : 19h…".
9. Frontend email drafts hardcode Issoire/Coubertin logistics + a personal phone number.
10. Rescue-tank accounting (+1 bottle/regulator/stab "Secours") is a club convention baked into the summary.
11. FFESSM-only level ladder; no licence number, no medical certificate, no other federations.
12. `groups` fixed at 3 seeded rows.
13. Single `config.json` loaded from CWD-relative path; no env overrides.

---

## 6. Technical debt, bugs, smells

### Likely real bugs

1. **`get_by_token`/`submit` look up the questionnaire by `person_id` only** (no session/sortie filter, `.one()`): for anyone registered to several sessions the magic link reads/overwrites an arbitrary questionnaire. Most damaging bug found.
2. `submit` never persists `nitrox_base_formation`/`nitrox_confirmed_formation`/`comes_from_issoire` although the request carries them — the public questionnaire cannot record "je pars d'Issoire" (silently keeps default `true`).
3. `submit` on a not-yet-created questionnaire returns 404 ("simplified version") — magic links only work for pre-imported people.
4. Rule R3 dropped from the update/create business-rule copies (drifted triplication).
5. `rotation_order.rs` doc comment contradicts the code.
6. `email_jobs.sortie_id` never written ⇒ sortie magic links always None.
7. `get_sortie_questionnaires` returns the raw token as `magic_link` (not a URL).
8. `create_sortie` indexes `time_labels[dive_of_day]` — panics if validation ever relaxed past 4.
9. `add_dive_director` doesn't check `is_encadrant` (fosse DP path does).
10. `update_palanquee` / `update_person` / `update_sortie` can never clear an optional field (Some-only semantics).
11. `update_skill_validation` ignores impersonation for validator attribution; `create` honours it.
12. `get_person_competencies` "is self" check uses the real email, not the impersonated one.
13. `magic_link.expiration_hours` config is dead (parameter `_expiration_hours`).
14. Import creates people with all-false gear defaults; `create_person` defaults regulator/stab true.
15. `copy_attendees` reports success without doing anything.

### Security issues

16. **Fail-open ACL**: unknown email ⇒ all permissions; person with no group ⇒ all permissions.
17. **Most routes have authentication but no authorization**: any logged-in user can create/delete sessions, CRUD people, **rewrite group permissions**, upload/delete level documents, generate anyone's PDF, mutate palanquées, download fiches, read everyone's magic links via `questionnaires-detail`, import CSVs.
18. **Public session summary leaks each participant's magic link + email** — anyone with the recap URL can alter every participant's questionnaire.
19. Google ID-token verification via `tokeninfo`, `aud` check only.
20. `CorsLayer::permissive()` in production.
21. JWT validation disables `required_spec_claims`.
22. `is_admin` embedded in JWT, trusted for full 24 h TTL; no revocation.
23. No rate limiting on login; 6-char temp password (32^6 ≈ 10^9) brute-forceable in the 24 h window; no lockout.
24. `must_change_password` is advisory — API never forces the change.
25. Email template interpolation without HTML escaping (stored XSS into `email_jobs.email_body`, rendered by the admin UI with `dangerouslySetInnerHTML`).
26. `director_accessible_sortie_ids` over-grants (any past encadrant → DP tools on next sortie).
27. Plaintext secrets in `config.json`; no env/secret-manager path.

### Data-model / consistency smells

28. Two DP mechanisms with different cardinality and validation, resolved by a preference chain in the PDF generator.
29. Two competency models both routed.
30. `diving_level` comma-string parsed ≥5 different ways; no validation on write.
31. `skill_validations` keeps only latest state, yet "logs" is presented as audit.
32. Missing constraints: no XOR CHECK on questionnaire session/sortie; no unique `(sortie_id, person_id)`; no unique rotation/palanquée numbers; `role`, `gas_type`, `sortie_type`, statuses all free strings.
33. Migration hygiene: fake date prefixes, a no-op "recreate people" migration, `let _ =` swallowed errors, add-then-remove column churn, lossy `down()`s.
34. N+1 queries throughout (palanquée tree, fiche, validations, hierarchy).
35. No transactions anywhere (sortie creation, group-permission replace, import).
36. Errors downgraded to `Custom("Failed to query X")`, while `AppError::Database` leaks `self.to_string()` into response `details`.
37. Massive response-mapping duplication (QuestionnaireResponse built by hand 7×).
38. Dead code: unused middleware, `require_permission!` macro, `GroupType::default_permissions`, `CsvImportRow::validate`, three unrouted email send methods, `sorties.summary_token`.
39. CWD-relative paths (`../frontend/dist`, `email_template.html`, `../config.json`).
40. Hand-rolled PDF content streams, two divergent accent tables, no text-width measurement.
41. `people.default_is_encadrant` written but never meaningfully read.

### Test coverage — effectively zero

- `tests/unit/` and `tests/integration/` have no `main.rs` / top-level `.rs` ⇒ **never compiled or run**; the code inside no longer compiles against current APIs; `fixtures/test_config.json` is stale (missing `smtp`).
- No insta `.snap` files exist despite snapshot assertions.
- What runs: 17 pure-logic unit tests inside `src/` (acl models, diving_level, middleware helpers). Zero HTTP/DB/auth/palanquée/PDF/import coverage.
- CI: Postgres 15 service, `cargo test` (passes vacuously), release build, SCP deploy. No clippy, no fmt, no frontend tests.

---

## 7. External integrations

- **Google OAuth**: code flow (token + userinfo) and One Tap (`tokeninfo`); new `reqwest::Client` per call. `GET /auth/config` exposes client_id to the SPA.
- **SMTP** (lettre, rustls, implicit TLS 465): fresh transport per email; only the temp-password email is actually sent.
- **PDF** (lopdf 0.34): fiche de sécurité built from scratch; level documents = uploaded template + FreeText annotations; `get_page_dimensions` for the placement UI.
- **CSV**: csv crate, headers required, UTF-8 only, no BOM handling. Two divergent sample files in the repo.
- **Config shape**: `server{host,port}`, `database{url}`, `admin{emails[]}`, `google_oauth{client_id,client_secret,redirect_uri}`, `jwt{secret,expiration_hours}`, `magic_link{base_url,expiration_hours(unused)}`, `smtp{host,port,username,password,from_email,from_name}`. All fields mandatory.
- **Database**: PostgreSQL 15 (docker-compose 5433→5432). Migrations run automatically at boot (`Migrator::up`); no separate migration binary.
