# Fosse Frontend — Functional Analysis

> Analysis of the reference application's frontend (`../fosse/frontend`),
> performed 2026-08-17. Feeds `DOMAIN.md` and `MODULES.md`. React 18 +
> TypeScript + Vite + TailwindCSS + zustand + react-router-dom v6 + axios +
> recharts + pdfjs-dist. ~17k lines, 24 pages, 8 shared components, 4 lib
> modules, 1 context. No test files, no i18n layer (all strings hard-coded
> French).

---

## 1. Pages & Navigation

### 1.1 Route table

Router is configured in `fosse/frontend/src/App.tsx` (top level) and
`fosse/frontend/src/pages/Dashboard.tsx` (nested `/dashboard/*`). `basename` is
`/fosse` in production, `''` in dev.

| Route | Component | Audience | Purpose |
|---|---|---|---|
| `/login` | `pages/LoginAdmin.tsx` | anonymous | Email+password login, Google OAuth / One Tap, "première connexion" & "mot de passe oublié" (request temp password by email) |
| `/change-password` | `pages/ChangePassword.tsx` | authenticated, forced when `must_change_password` | Set new password (min 8 chars + strength meter) |
| `/q/:token` | `pages/PublicQuestionnaire.tsx` | **anonymous / magic-link visitor** | Fill & submit the pool-session questionnaire |
| `/s/:token` | `pages/PublicSummary.tsx` | **anonymous / magic-link visitor** (shareable summary token) | Read-only session recap: gear/bottle counts, participant lists |
| `/` | → redirect `/login` | — | — |
| `/dashboard` | `pages/DashboardHome.tsx` | all authenticated | Personal home: profile-completeness warning, stat tiles, upcoming events, quick actions |
| `/dashboard/sessions` | `SessionsPage` if admin, else `MySessionsPage` | admin / member | Admin: CRUD sessions + questionnaires. Member: "Plongées à venir" + past dives |
| `/dashboard/fosse-session/:sessionId/participants` | `FosseSessionParticipantsPage.tsx` | DP of a pool session, admin | Targeted participants/questionnaire management for non-admin DPs (SessionsPage equivalent, scoped) |
| `/dashboard/sorties` | `SortiesPage.tsx` | admin (full list) / DP+encadrant (restricted list from `/me/sorties-director-access`) | List & create sorties |
| `/dashboard/sorties/:id` | `SortiePage.tsx` | admin / sortie DP (read-only for others with access) | Sortie detail: dives, DPs per dive, registered participants |
| `/dashboard/sorties/:id/fiches` | `SortieFichesPage.tsx` | admin / sortie DP only | "Bilan": enter *realised* time & depth per palanquée, then download fiche de sécurité PDF per dive |
| `/dashboard/summary/:id` | `SummaryPage.tsx` | admin only (route gated) | Full session recap + CSV export |
| `/dashboard/palanquees/:sessionId` | `PalanqueesPage.tsx` | everyone registered (backend-enforced); **edit** only admin or DP | Build rotations & palanquées via drag-drop / tap, fiche de sécurité PDF |
| `/dashboard/import` | `ImportPage.tsx` | admin only | CSV import of divers into a session (**no nav link — orphan route**) |
| `/dashboard/emails` | `EmailsPage.tsx` | admin only | Copy-paste email helper (recipients, subject, invitation body, gear-request body) |
| `/dashboard/emails/session/:id` | `SessionEmailsPage.tsx` | admin only | Legacy email-job queue: preview/mark-as-sent generated magic-link emails (**orphan, no link anywhere**) |
| `/dashboard/users` | `UsersPage.tsx` | admin only | People CRUD, diving-level editor, impersonation |
| `/dashboard/profile` | `MyProfilePage.tsx` | all (nav shows it for non-admins) | Theme, phone, default gear preferences (level read-only) |
| `/dashboard/competences` | `CompetencesAdminPage` (admin) / `CompetencesInstructorPage` (can validate) / `MyCompetencesPage` (member) | role-dispatched at the route level | Competency admin / validation entry point / own progression |
| `/dashboard/competences/student/:studentId` | `StudentCompetencesPage.tsx` | admin + encadrants | Validate a student's skills |
| `/dashboard/sortie-groupes` | `SortieGroupesPage.tsx` | admin + encadrants | Filter a sortie's students by validated skills / stage to constitute level groups |
| `/dashboard/groups` | `GroupsPage.tsx` | admin only | Permission matrix per group |
| `/dashboard/level-documents` | `LevelDocumentsPage.tsx` | admin only | Upload level PDF templates and visually place each skill's stamp box |
| `/dashboard/validation-logs` | `ValidationLogsPage.tsx` | admin only | Audit log of all skill validations |
| `/dashboard/competences-legacy` | `CompetencesPage.tsx` | admin only | Old flat competency model (**orphan, superseded**) |
| `*` | → redirect `/dashboard` | — | — |

### 1.2 Navigation & role model

`components/Header.tsx` renders desktop nav + a mobile hamburger menu with
emoji-prefixed items. Visible items:

- Always: Tableau de bord, Sessions, Compétences.
- `showSortiesNav` (from `contexts/SortiesAccessContext.tsx`): admin, **or** non-admin whose `/me/sorties-director-access` returns ≥1 sortie.
- Admin: Utilisateurs, Groupes, Documents, Logs, Emails. Non-admin: Mon Profil instead of Utilisateurs.
- Admin **or** `canValidateCompetencies`: "Groupes sortie".

Three displayed roles (badge in header): **Admin** (red), **Encadrant**
(orange, `can_validate_competencies`), **Membre** (cyan). Impersonation adds a
red inset ring on the whole dashboard plus a fixed banner
(`components/ImpersonationBanner.tsx`).

---

## 2. User Workflows (step by step)

### 2.1 Member answers a questionnaire (magic link, anonymous)
`pages/PublicQuestionnaire.tsx`
1. Member receives an email with `{base}/q/{token}` (token generated server-side at CSV import / `generate-links`).
2. Page loads `GET /questionnaires/by-token/:token`; shows "Lien invalide" on failure, and a "déjà soumis" panel if `questionnaire.submitted_at` is set (no editing possible from the public page).
3. Form is pre-filled from an existing questionnaire. Conditional field logic:
   - "Je suis encadrant" → reveals *2ème détendeur* and *nitrox*; unchecking it force-resets both to false.
   - Not encadrant → reveals "Je participe à la formation Nitrox (bloc nitrox)".
   - "J'ai besoin d'une stab" → required size select XS…XL.
   - "Je pars d'Issoire" → reveals "covoiturage" → reveals `car_seats` with an explicit warning **"Comptez-vous dans les places disponibles"** (driver included).
4. `POST /questionnaires/submit` with `nitrox_base_formation`/`nitrox_confirmed_formation` hard-coded to `false` (those are sortie-only fields).
5. Success toast; the page then just sits there (a `setTimeout` with an empty body — "could navigate to a thank-you page").

### 2.2 Member answers from inside the app (logged in)
`pages/MySessionsPage.tsx` → `RegistrationDetails` sub-component.
- Only shown when: registered **and** not a sortie dive **and** `isCoubertinClubFosseLocation(session.location)` — i.e. gear/transport is only editable for the club's own pool (Coubertin, or a legacy blank location).
- Fields: détendeur, stab (+size), nitrox & 2ᵉ détendeur (encadrants only), transport as a **radio pair** "Départ Issoire" / "Départ Clermont", car + seats (defaults to 4 when checked).
- "✅ Valider" calls `PUT /questionnaires/:id` with `mark_as_submitted: true`, then full page reload of data. Status shows "Réponse validée / non validée".

### 2.3 Admin creates a pool session (fosse)
`pages/SessionsPage.tsx`
1. "Créer une session" modal: **Lieu** select limited to `Coubertin | Montluçon | Le Puy-en-Velay`; date; the **name is auto-generated and the field is disabled** (`"{lieu} - {DD/MM/YYYY}"`); optional CSV of participants.
2. `POST /sessions`, then if a CSV was attached, immediately `POST /import` with the new session id (import creates people, questionnaires, magic links and email jobs server-side).
3. From the session list row: "Questionnaires" (inline detail view), "🤿 Palanquées", "📊 Récap", "🗑️ Supprimer" (confirm warns that questionnaires + emails are cascade-deleted).
4. In the detail view: toggle **"🔄 Mode 2 rotations"** (`optimization_mode`, hidden for partner pools), pick the **DP** from a select restricted to `is_encadrant` questionnaires, "➕ Ajouter un participant".
5. Per-participant card: edit (modal), delete, toggle submitted/not-submitted (re-sends the whole questionnaire payload with `mark_as_submitted`), copy magic link, view `email_status`.

### 2.4 Admin creates a sortie (dive trip)
`pages/SortiesPage.tsx`
1. Modal: type `exploration | technique`; free-text lieu; start date; `days_count` (1–7) × `dives_per_day` (1–4) with a live "Total: N plongées seront créées"; `nitrox_compatible` checkbox shown **only for technique**; auto-generated disabled name `"Sortie {Technique|Exploration} - {lieu} - {date}"`; optional description.
2. `POST /sorties` creates the sortie **and** its N dives (each dive is a `Session` with `sortie_id` + `dive_number`); toast reports the dive count; navigates to the sortie page.
3. `pages/SortiePage.tsx`: stats tiles (plongées / participants / encadrants / inscrits), dives table with **up to 4 DPs per dive** (`+ DP` button, add/remove via `dive-directors` endpoints, chooser lists encadrant questionnaires not already DP), and a participants table. Participants can be added via `AddSortieParticipantModal` (search existing person or create; `is_encadrant` auto-derived from `person.is_instructor`; Nitrox Base / Nitrox Confirmé checkboxes only when technique+nitrox_compatible). Editing a participant only exposes the two nitrox-formation flags — role is read-only "(basé sur le niveau)".

### 2.5 Building palanquées and rotations
`pages/PalanqueesPage.tsx` (1 566 lines — the functional core).

Load sequence is deliberately fault-tolerant: session, palanquées, summary,
questionnaires (from the **sortie** if `session.sortie_id`, else from the
session), and dive-directors are each fetched in independent try/catch blocks;
failures degrade to read-only rather than erroring.

Edit permission (client-side): `canEdit = isAdmin || isCurrentUserDP`, where
`isCurrentUserDP` = my questionnaire has `is_directeur_plongee` **or** appears
in `dive_directors`. Note the comment: admins keep edit rights **even while
impersonating**, so an impersonated DP can manage palanquées.

Steps:
1. Create rotations. On partner pools (`isExternalClubGearLocation`) the buttons are "➕ Rota · Plongée 1" and "➕ Rota · Plongée 2" (`plongee_number` 1|2); elsewhere a single "➕ Rotation".
2. Inside a rotation, "+ Palanquée" creates an empty palanquée card with a **GP zone (max 2)** and an **Élèves zone (max 4)** (`MAX_GPS = 2`, `MAX_STUDENTS = 4`).
3. Assign people from the left "Non assignés" panel, grouped as: 🏅 Encadrants, ⚡ Formation Nitrox, 🔥 Nitrox Confirmé, 📘 Prépa PESH6/PESH12/N1/N2/N3/N4, 📋 Autres.
   - Desktop: HTML5 drag & drop onto the GP or Élèves zone.
   - Mobile: **tap-to-select** (sticky cyan banner shows the selection) then tap the target zone; "+ Ajouter ici" hints appear on `sm:hidden`.
4. Guard rails enforced client-side before the API call: GP zone full → `alert('Cette palanquée a déjà 2 guides de palanquée')`; already placed in this rotation → `alert('Ce participant est déjà dans une palanquée de cette rotation')` (`participantInRotation`).
5. Gas type is **inferred, never asked**: `gasType = (wants_nitrox || nitrox_training) ? 'Nitrox' : 'Air'` at `handleAddMember`.
6. Per-palanquée ⚙️ panel: *planned* duration (0–120 min) and depth (0–60 m), saved via `PUT /palanquees/:id`; a cyan strip shows `⏱️ 40'` / `📏 20m` when set.
7. Bottle budget tracking per rotation (club pool only, hidden for sorties and partner pools): `Air: used/available`, `Nx: used/available`, turning red + ⚠️ and red card border when exceeded.
8. Removal: ✕ on a member, 🗑️ on a palanquée or rotation (both with `confirm()`).

### 2.6 Fiche de sécurité PDF
Two entry points, both `GET /sessions/:id/fiche-securite` returning a blob
turned into a download link.

- **Per session** (`PalanqueesPage` modal): fields Date, Club (default `"USI Plongée"`, placeholder `"PALME Issoire"`), Site (pre-filled from `session.location`), Position, Sécurité Surface, Observations; on partner pools an extra **"Portée du PDF"** select (`Toutes les plongées | Plongée 1 | Plongée 2` → `plongee_number` param). File name `Fiche_Securite_{session}[_PlongeeN].pdf`.
- **Per sortie "Bilan"** (`SortieFichesPage`): a shared PDF header form (Club/Position/Sécurité surface/Observations), then per dive a list of rotations → palanquées where the DP types **realised** duration (0–180 min) and depth (0–120 m). Helpers `isPalanqueeComplete` / `countIncompletePalanquees` drive an amber "(N palanquées incomplètes)" or a green "✓ Prêt pour PDF" indicator, and a per-palanquée amber background. "Enregistrer tout" persists all forms; "📥 PDF" first persists everything, then downloads — and if some palanquées are incomplete it asks `confirm("… Télécharger quand même le PDF ?")`. File name `Fiche_Securite_{sortie}_{diveNumber}.pdf`.

### 2.7 Competency validation
- **Structure (admin)** — `pages/CompetencesAdminPage.tsx`, tab `📚 Hiérarchie`: level tabs N1/N2/N3 → Domaines → Modules → Acquis, each CRUD-able through modals. A skill has name, optional description ("aide affichée aux encadrants"), sort order and **`min_validator_level`** chosen from `['E2','E3','E4','N4','E1','N3']`. Expanded-branch state is lifted to the page so it survives reloads, and creating a child auto-expands its parents.
- **Stages (admin)** — tab `🔄 Étapes`: CRUD of `ValidationStage` (code, name, description, colour picker, emoji icon, sort order, `is_final`). Example given in the UI: "Vu en piscine → Acquis → Validé en mer".
- **Validation (encadrant/admin)** — `CompetencesInstructorPage` (level tabs PESH6/PESH12/N1/N2/N3, students who `preparing_level === tab`) → `StudentCompetencesPage`:
  1. Header with student name, prepared level, and 4 stat tiles (fosses attended, validées, en cours, dernière fosse).
  2. Domains auto-expanded; each skill row shows a row of stage buttons coloured by `stage.color`.
  3. **Monotonic progression rule (client-side)**: `canMoveToStage` disables (opacity 20 %, `cursor-not-allowed`) any stage whose `sort_order` is lower than the current one — *unless* `isRealAdmin` (`isAdmin && !impersonating`).
  4. Clicking a stage opens an ⚠️ irreversibility confirmation modal, then `POST /skill-validations`.
  5. A "pending changes" batch mode exists (`pendingChanges` map + `💾 N modif.` button doing `Promise.all` of creates) but is only reachable once a pending change exists — clicking a stage from a clean state goes straight through the confirm dialog.
  6. Real admins get a 🗑️ per validation (`DELETE /skill-validations/:id`) with a `confirm()`.
- **Student view** — `MyCompetencesPage.tsx`: if no `preparing_level`, an empty state telling the member to contact an admin. Otherwise a global progress bar (green = validated, yellow = in progress), a stage legend, domain → module → skill listing with per-skill stage badge, `validated_at`, validator name and notes, plus a **"📥 Télécharger PDF"** button when a level template exists (`GET /level-documents/:level/generate/:personId`, filename `Competences_{level}_{NOM}_{Prénom}.pdf`).
- **Audit** — `ValidationLogsPage.tsx`: 4 stat tiles (total, acquis finaux, élèves concernés, encadrants actifs), free-text search across student/instructor/skill/module/domain, level filter, and a table.
- **Group building** — `SortieGroupesPage.tsx`: pick a sortie + prepared level, then filter students by name, by validation stage, and by a multi-select set of skills (semantics: keep students who have **all** selected skills validated at *any* stage). Each student card shows their validated-skill chips coloured by stage and a link to their fiche.

### 2.8 Imports
Two distinct paths, both hitting `POST /import` (multipart, `session_id` + `file`):
- `pages/ImportPage.tsx`: pick a session, upload a CSV with columns `first_name, last_name, email, phone(optional)`; the result panel shows total rows / success / errors with per-row messages and points to the Emails page. Reachable only by typing the URL.
- **Session creation** with an attached CSV (`SessionsPage`).
- **`AddParticipantModal.tsx` fabricates a one-line CSV in memory** (`new File([blob], 'participant.csv')`) and posts it to the import endpoint, for both "select existing person" and "create new person". This is how single participants are added to a pool session — there is no dedicated add-participant API call on that path. (Sorties use the proper `POST /questionnaires/register` instead, via `AddSortieParticipantModal`.)

### 2.9 Emails
The app **never sends** email itself; it is a copy-paste assistant.
- `pages/EmailsPage.tsx` (current): choose a fosse (auto-selects the next future session), then get (a) the recipient list with **your own address filtered out**, one-line comma-joined addresses + "Copier tous les emails"; (b) an auto-generated **subject** `"Fosse {lieu} - {date longue}"`; (c) a fully hard-coded **invitation body** (mentions "départ d'Issoire à 19h, rendez-vous à Coubertin aux alentours de 20h" and a **hard-coded personal phone number `06 63 90 35 21`**); (d) a **gear-request body** embedding the public summary URL `{origin}{base}/s/{summary_token}` or a "[Lien non disponible – générer les liens d'abord]" placeholder.
- `pages/SessionEmailsPage.tsx` (legacy, unreachable): the server-generated email-job queue — select rows, preview the HTML body (`dangerouslySetInnerHTML`) with Gmail copy-paste instructions, "Marquer comme envoyés", copy-all, CSV export. Backend job generation happens on import; `emailsApi.getPending` and `sessionsApi.generateMagicLinks` are declared in `lib/api.ts` but **called from nowhere in the UI**.

### 2.10 Impersonation
`UsersPage` → "👤 Impersonnifier" → confirm modal → `POST /auth/impersonate` →
`setImpersonation(token, info, can_validate_competencies)`. The whole dashboard
gains a red ring + banner; `isAdminView()` becomes false so the UI switches to
the target's role; `targetEmail = impersonating?.user_email || email` is used
everywhere to resolve "me". Exit via the banner (`POST /auth/stop-impersonation`).

### 2.11 Level document (PDF stamp) editor
`pages/LevelDocumentsPage.tsx`: pick a level (`N1 N2 N3 E1 N4 E2 E3 E4`),
upload/replace/delete a PDF template, render it client-side with **pdf.js**
(worker injected as a raw-text blob URL to dodge MIME issues), page navigation
and 50–200 % zoom. Then, for each skill of that level, click 📍 to enter
placement mode and **draw a rectangle** on the page; existing boxes can be
dragged and resized by their corner handles. Coordinates are converted between
canvas and PDF space (`y` flipped: `pdfY = pdfHeight - canvasY/scale`),
rounded, and saved with `font_size: 8`. The right panel splits skills into
"⏳ Non placés" and "✓ Placés (page, X, Y)".

---

## 3. Terminology & UX patterns

### 3.1 French diving vocabulary used in the UI
- **Fosse** = deep pool session; **Sortie** = sea/dive trip (`exploration` | `technique`); **Plongée** = one dive of a sortie (also "Plongée 1 / Plongée 2" on partner pools); **Rotation / Rota** = a wave of dives; **Palanquée** (abbrev. "Pal.") = a dive team.
- **DP / Directeur de Plongée** (single per fosse via `is_directeur_plongee`; up to 4 per sortie dive via `dive_directors`); **GP / Guide de Palanquée**; roles stored as `E`, `GP`, `P` (encadrant / guide / plongeur) — the UI groups `GP` and `E` together as "GP" and treats `P` as "Élèves".
- **Encadrant** vs **Élève**; **Plongeur**.
- **Niveaux**: PESH6, PESH12 (parcours adapté handicap), N1…N5, aptitudes PE40/PA20/PA40/PE60/PA60, encadrant grades E1…E4. Full names displayed: "Niveau 2 - Plongeur Autonome 20m", "Niveau 4 - Guide de Palanquée", "Niveau 5 - Directeur de Plongée".
- **Gaz**: Air / Nitrox ("Nx" badge), **bloc / bouteille**, **bloc de secours**, "Formation Nitrox" (base) and "Nitrox Confirmé".
- **Matériel**: détendeur, 2ème détendeur, **stab** (BCD, sizes XS–XL), bouteille.
- **Compétences**: Domaine → Module → **Acquis** (skill), **Étape de validation**, "min. validateur".
- **Fiche de sécurité**: Club, Site, Position, **Sécurité surface**, Observations, durée/profondeur prévue vs réalisée.
- Logistics: **Issoire** (club departure point) vs **Clermont**, covoiturage, places.

### 3.2 UX patterns
- **Auto-generated, disabled name fields** for sessions and sorties (lieu + date), with the field still displayed so the user sees the result.
- **Progressive disclosure** in every form: checkbox reveals dependent fields (stab→size, car→seats, encadrant→nitrox/2nd reg, technique→nitrox_compatible).
- **Toast** (`components/Toast.tsx`, bottom-right, 3 s auto-dismiss) for all feedback; native `confirm()`/`alert()` for destructive actions and drag-drop rejections (inconsistent with the Modal component that also exists).
- **Card grids over tables** for people/students; `components/Table.tsx` (generic column renderer) used for sessions, sorties, dives and sortie participants.
- **Level tabs** with badge counts (domains, students) on all competency pages.
- **Coloured stage chips** built from `stage.color + '20'` background / `+ '40'`/`'60'` border — a repeated inline-style idiom.
- **Emoji as iconography** throughout (🤿 📅 🏅 ⚡ 🎯 ✅ ⏳ 🗑️ 📥), including in nav and stage icons which are user-editable text fields.
- **Dark theme by default** with a light theme; implemented with CSS variables + `theme-*` utility classes in `src/index.css`, toggled by adding `.dark`/`.light` on `<html>` (`lib/theme.ts`, persisted, applied at module load to avoid flash). Many older components still hard-code `bg-slate-800/50` instead of using `theme-*` (e.g. `SessionEmailsPage`, `AddSortieParticipantModal`, parts of `CompetencesInstructorPage`).
- **Mobile considerations** are real and non-trivial: hamburger nav with role badge; `PalanqueesPage` tap-to-assign replacing drag & drop with a sticky selection banner and "+ Ajouter ici" drop hints; `sm:`/`min-[400px]:` responsive text truncation everywhere (`hidden sm:inline` on button labels); reduced paddings/font sizes on `StudentCompetencesPage`, `MyCompetencesPage`, `UsersPage`; Google **One Tap** explicitly for "connexion rapide sur mobile".
- **Name formatting convention**: `LASTNAME F.` (`last_name.toUpperCase()` + first initial) in palanquées and fiches, full names elsewhere.
- **Fullscreen chart affordance** (`ChartCard` in `CompetencesAdminPage`) with an overlay dialog.

---

## 4. Client-side state & API usage

### 4.1 API layer
`src/lib/api.ts` (859 lines) is the single axios instance and the whole type
surface. Base URL is derived from `import.meta.env.BASE_URL` →
`/fosse/api/v1` in prod, `/api/v1` in dev (Vite proxies to `localhost:8080`).

- Request interceptor injects `Authorization: Bearer` from `localStorage.auth_token`.
- Response interceptor on **401**: clears `auth_token` + `auth-storage` and hard-redirects to `{base}/login` — **except while impersonating**, where 401s are swallowed (the admin token is still valid, the impersonated user simply lacks the permission).
- Grouped API objects: `authApi`, `sessionsApi`, `sortiesApi`, `questionnairesApi`, `importApi`, `emailsApi`, `peopleApi`, `validationStagesApi`, `competencyDomainsApi`, `competencyModulesApi`, `competencySkillsApi`, `skillValidationsApi`, `competenciesApi` (legacy), `groupsApi`, `levelDocumentsApi`, `palanqueesApi`.
- Blob endpoints: `downloadFicheSecurite`, `levelDocumentsApi.download`, `generateFilled`.

### 4.2 State
- **zustand + persist** for auth (`lib/auth.ts`, key `auth-storage`) and theme (`lib/theme.ts`, key `theme-storage`). Token is stored **twice**: inside the persisted store and separately in `localStorage.auth_token`.
- Auth store fields: `token, email, name, isAuthenticated, isAdmin, canValidateCompetencies, impersonating, mustChangePassword`, plus derived getters `isAdminView()` (`isAdmin && !impersonating`) and `canValidate()`.
- **One React context**: `SortiesAccessContext` — fetches `/me/sorties-director-access` once for non-admins and exposes `showSortiesNav`, `directorSorties`, `loadingDirectorSorties`. Used to gate the Sorties nav item, the sortie list content, DP tools on `SortiePage`, and the whole `SortieFichesPage`.
- Everything else is local `useState` per page, with imperative `loadData()` functions re-called after every mutation (no cache, no react-query, no optimistic updates).

### 4.3 Business rules that live only in the frontend

Several rules exist nowhere but in JSX — critical for the rewrite:

1. **Bottle/tank computation with "mode 2 rotations"** — duplicated verbatim in `SummaryPage.tsx` (l. 175–256), `PublicSummary.tsx` (l. 107–197) and `PalanqueesPage.tsx` (twice: l. 736–772 and l. 795–812). The formula:
   - `studentsAir = students_count - nitrox_training_count`; `backupTank = 1` (bloc de secours, always Air).
   - If `optimization_mode`: `ceil((studentsAir + 1)/2)` and `ceil(studentsNitrox/2)` — students do two rotations on the same blocks.
   - `nitroxBottles = encadrants_nitrox + optimizedStudentNitrox`; `airBottles = (encadrants_count - nitrox_count) + optimizedStudentAirPlusBackup`; total = sum. "Économisées" counters derive from the difference.
   The backend's `total_bottles`/`nitrox_bottles`/`air_bottles` fields exist in `SessionSummary` but the UI **ignores them** and recomputes.
2. **Location-driven feature flags** — `src/lib/fosseLocations.ts`:
   - `EXTERNAL_CLUB_GEAR_LOCATIONS = ['Le Puy-en-Velay', 'Montluçon']` → hides all air/gear/bottle sections (summaries, palanquée counters), switches palanquées to a "Plongée 1 / Plongée 2 → Rota k" structure, enables the PDF scope selector, and disables the "mode 2 rotations" toggle.
   - `isCoubertinClubFosseLocation()` → `'Coubertin'` **or empty string** ("anciennes sessions sans lieu : comportement historique"). Gates the in-app gear/transport form in `MySessionsPage` and the "2ᵉ rot." per-student toggle in `PalanqueesPage`.
   These are hard-coded string constants matched against a hard-coded `<select>` in `SessionsPage` — adding a pool means editing two files.
3. **Palanquée composition limits** — `MAX_GPS = 2`, `MAX_STUDENTS = 4`, and "one participant per rotation" (`participantInRotation`) are enforced in the browser (the backend may also enforce them; the UI shows `alert()` before calling).
4. **Gas assignment** — `Nitrox` iff `wants_nitrox || nitrox_training`, decided at drop time.
5. **Diving-level parsing and display** — `diving_level` is a **comma-separated string** including `preparing_*` tokens (e.g. `"N1,N2,preparing_N3"`). `PalanqueesPage` holds `LEVEL_HIERARCHY` (PESH6=0 … E4=14), `getHighestLevel`, `getHighestLevelExcludingInstructors`, and `displayEncadrantLevelBadge` — the last encodes a very specific club rule: *"for an **E1** show their diving level (N2/N3…) instead of E1; E2–E4 always show their grade as-is"*. `UsersPage` has a **second, different** rank table (`LEVEL_RANK`, PESH6=6 … E4=70) and `AddSortieParticipantModal` has a **third, naive** heuristic (`diving_level.split(',').pop()`).
6. **Level cascade & preparing-level eligibility** — `DivingLevelModal` in `UsersPage.tsx`: checking a classic level auto-checks all lower ones and unchecking cascades upward; PESH12 implies PESH6; the two paths are independent. `getPreparingOptions()` computes the *only* allowed next level from the max validated rank (no validated level → PESH6/N1/N2; PESH6 → PESH12 or N1; PESH12 → N1; otherwise strictly the next entry of `['N1','N2','N3','E1','N4','N5','E2','E3','E4']`). Saving re-serialises everything into the comma string with a `preparing_{level}` token. "Encadrant" is previewed as *E2 or above*, while the backend exposes `is_instructor` separately.
7. **Competency stage monotonicity** — `canMoveToStage` in `StudentCompetencesPage`: non-admins can only move forward (`targetStage.sort_order >= currentSortOrder`); real admins bypass it and alone may delete validations.
8. **DP/edit-permission inference** — `canEdit` in `PalanqueesPage`, `canUseSortieDirectorTools` in `SortiePage`, `canEdit` in `SortieFichesPage`; all derived from client data (`emailsMatch` on lowercased trimmed emails, membership in `directorSorties`).
9. **Student grouping taxonomy** — the ordered buckets "Formation Nitrox → Nitrox Confirmé → par niveau préparé → autres" and the level list `['PESH6','PESH12','N1','N2','N3','N4']` are repeated in `SummaryPage`, `PublicSummary` (which uses only `['N1','N2','N3','N4']` — inconsistent) and `PalanqueesPage`.
10. **Email content templates** (subject, invitation, gear request) are pure frontend strings in `EmailsPage.tsx`.
11. **Fiche completeness rule** — a palanquée is "complete" iff both realised time and depth are non-empty numbers (`SortieFichesPage`).
12. **CSV export shape** — `SummaryPage.exportCSV` builds the 12-column CSV client-side (naive quoting, no escaping of embedded quotes).
13. **Sortie name / session name conventions**, and the statistics pages parsing the session **name** with a regex `/(\d{2}\/\d{2}\/\d{4})/` to get an X-axis label.
14. **"Which dives do I see"** rule in `MySessionsPage`: for a sortie you're registered to, you see **only the first upcoming dive** unless you are DP of one of them, in which case you see all of them.
15. **Login mode switching** — if the typed email ends with `@gmail.com`, the password form is replaced by a Google-only flow ("Les comptes Gmail utilisent la connexion Google").

---

## 5. Frontend-only features / affordances worth preserving

- **Copy-paste email cockpit** (`EmailsPage`): recipient list minus yourself, generated subject, two ready-made bodies, public summary link. Entirely frontend; no backend counterpart in use.
- **Public summary token page** (`/s/:token`) — shareable, no login, with the "valide jusqu'au lendemain de la session" expiry explanation, used by the gear-request email.
- **Drag & drop + mobile tap-to-assign palanquée builder**, including live bottle-budget badges that turn red on overrun.
- **Bilan workflow** (`SortieFichesPage`): batch entry of realised parameters with completeness indicators and a "download anyway?" guard before the PDF.
- **Visual PDF stamp-placement editor** with pdf.js, drag/resize handles and coordinate conversion.
- **Impersonation with unmistakable visual framing** (red ring + banner) and 401 suppression so the admin doesn't get logged out.
- **Diving-level modal** with hierarchical cascade, PESH parallel track, live "Aperçu du résultat" and constrained next-level selection.
- **Profile-completeness nudge** on `DashboardHome` (missing phone or missing level → CTA).
- **Past dives recap** on `MySessionsPage`: your past palanquées, teammates as chips, and for instructors a clickable shortcut from a student chip straight to their competency validation page (`🎯N2` badge marks students in training).
- **Skill-based group builder** (`SortieGroupesPage`) — multi-skill AND filter to constitute homogeneous groups for a trip.
- **Auto-generated names**, auto-selection of the next upcoming session, auto-expansion of newly created hierarchy nodes, persisted expanded state, fullscreen charts.
- **Theme switcher** (dark default) with FOUC-free initialisation.
- **Magic-link copy buttons + per-participant `email_status`** display in the admin session view.

---

## 6. Pain points / technical debt

**Duplication**
- The bottle/optimisation formula exists in 4 places (2 of them inside `PalanqueesPage` itself).
- `SummaryPage` and `PublicSummary` are near-identical (~600 lines duplicated: StatCard, StudentTable, grouping, bottle maths) differing only by theme classes and the preparing-level list.
- `CompetencesAdminPage.StatisticsSection` and `CompetencesInstructorPage.StatisticsSection` are copy-pasted (~250 lines), one with `ChartCard`/fullscreen and validation-log chart, the other without.
- `SessionsPage` and `FosseSessionParticipantsPage` duplicate the entire participant-card UI + toggle-status + DP selector logic.
- Three different level-ranking implementations (`LEVEL_HIERARCHY`, `LEVEL_RANK`, `split(',').pop()`), and 5+ divergent `LEVEL_ORDER` / `LEVEL_NAMES` constants (`['N1','N2','N3']` in admin, `['PESH6','PESH12','N1','N2','N3']` in instructor, `['PESH6',…,'E4']` in legacy, `['PESH6','PESH12','N1','N2','N3']` in `SortieGroupesPage`, `['N1','N2','N3','N4']` in `PublicSummary`).
- `EmailToSend` interface re-declared in `SessionEmailsPage` although it is exported from `lib/api.ts`.

**Data-fetching / performance**
- `MySessionsPage.loadData()` is an N+1+M waterfall: list all sessions, list all sorties, then per sortie fetch questionnaires, then per sortie fetch detail, then per dive fetch dive-directors, then per registration fetch palanquées and dive-directors again, then the same for every past session. Dozens of sequential awaits on a normal account.
- `DashboardHome` loops over **every** session calling `questionnairesApi.list` just to count "Mes Fosses".
- `StudentCompetencesPage` loops over all sessions to count attendance.
- Statistics sections cap work with magic numbers (`slice(-10)` sessions, `slice(0, 20)` students "pour la perf") — silently wrong aggregates.
- `PalanqueesPage` reloads the entire page dataset (`loadData()`) after every single member add/remove — heavy and loses scroll/selection context.
- `CompetencesAdminPage.loadData` re-fetches all people + stages + domains + a request per domain, on every level tab change and after every mutation.

**Error handling / UX consistency**
- Mix of `Toast`, native `alert()`, native `confirm()`, and inline error divs. Destructive palanquée/rotation deletes use `confirm()` while user deletion uses a proper `Modal`.
- Many `catch {}` blocks silently swallow errors ("Ignorer"), so partial data looks like real data (e.g. a session where palanquées failed to load renders as "no palanquées").
- `console.log` left in production paths, including `Dashboard.tsx` logging the auth state on every render and `UsersPage` logging the impersonation response.

**Routing / dead code**
- `/dashboard/import`, `/dashboard/emails/session/:id` and `/dashboard/competences-legacy` have **no links anywhere**.
- `ImportPage` uses a raw `<a href="/dashboard/emails">`, which breaks under the `/fosse` basename in production.
- `LoginAdmin.handleGoogleFallback` hard-codes `${origin}/fosse/login` as redirect URI (broken in dev), while `lib/auth.initiateGoogleLogin` computes it correctly — and that correct function is never called.
- Unused API surface: `sessionsApi.generateMagicLinks`, `emailsApi.getPending`, `sortiesApi.copyAttendees`, `palanqueesApi.listRotations`, `palanqueesApi.updateMember`, `levelDocumentsApi.batchUpdatePositions`, most of `competenciesApi`.
- Duplicated DOM id `google-signin-button` rendered in both branches of the login form (both `hidden`), so `renderGoogleButton` targets a hidden element.

**Security / correctness smells**
- Authorisation is largely re-implemented client-side (`canEdit`, `isAdminView`, route gating). The routes themselves are conditionally mounted, so a non-admin gets redirected rather than 403'd — fine, but the real rules must be mirrored server-side.
- `dangerouslySetInnerHTML` on server-provided email bodies (`SessionEmailsPage`).
- Token duplicated in two localStorage keys; logout removes both but the persist middleware rewrites `auth-storage`.
- 401 suppression during impersonation means genuine expiry while impersonating leaves the user in a broken state.
- `PublicQuestionnaire` submits `nitrox_base_formation: false` / `nitrox_confirmed_formation: false` unconditionally — resubmitting a sortie-linked questionnaire through that path would erase those flags.
- Toggle-submitted handlers resend the *whole* questionnaire payload read from stale list state (lost-update risk).
- Hard-coded personal phone number and club-specific logistics text inside `EmailsPage.tsx`.

**Typing / style**
- Frequent `any` (`onSave: (id: string, data: any) => Promise<void>`, `error: any`, `peopleApi.create(formData as any)`, `page.render({...} as any)`), and `@ts-ignore` for the pdf.js worker import.
- Mixed import styles (`@/lib/api` alias vs `../lib/api` in `PalanqueesPage`).
- `StudentTable` and `StatCard` components are re-declared inline inside render functions (`SummaryPage`), remounting on every render.
- `CompetencesAdminPage.StatisticsSection` destructures `{ people }` but the props type also declares `stages` (unused), and `setProgressData(progressByLevel as any)`.
- `PalanqueesPage` (1 566 lines) and `CompetencesAdminPage` (1 615 lines) are single-file god components mixing page logic, sub-components, and modals.
- Half-migrated theming: `theme-*` CSS utilities coexist with hard-coded `bg-slate-*`/`text-white` and with per-component `theme === 'dark' ? … : …` ternaries (`DashboardHome`, `MyProfilePage`, `Header`), so light mode is visibly broken on several pages (e.g. `EmailsPage` blue info card, `SessionEmailsPage`, `AddSortieParticipantModal`).
