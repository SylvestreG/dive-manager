# Module `users` — design (profil plongeur global)

Status: **draft for review** (2026-08-17). Décision produit fondatrice : le
profil plongée (certifications, expérience, niveaux) appartient à la
**personne**, pas au club — les brevets sont fédéraux et suivent le plongeur.
Conséquence structurante : **user = la personne** (la table `users` de la
fondation accepte déjà des comptes jamais connectés, statut `invited`) — le
concept « profil membre sans compte » du design initial disparaît.

## 1. Responsabilité du module

Étendre la personne d'un **profil plongeur global** : état civil minimal
(date de naissance), coordonnées, certifications FFESSM datées avec
**provenance**, statuts de formation datés, validité du CACI, numéro de
licence, préférences matériel, brevet en préparation. Exposer les **données
dérivées** via la lib `ffessm` (aptitudes exerçables, grade d'encadrement,
prochains brevets possibles) — calculées à la lecture, **jamais stockées**.

Hors périmètre : le carnet de plongées détaillé (viendra avec `activities`),
les arbres de compétences (module `competencies`, club-scopé), l'écriture de
la table `users` elle-même (réservée à `identity`).

## 2. Règles métier

1. **Propriété** : le profil appartient au user. **Visibilité** : lui-même,
   les clubs où il a une **adhésion active**, les admins globaux (audité).
2. **Provenance des certifications** — chaque entrée porte qui l'affirme :
   `declared` (auto-déclarée — **réservé** : voir décision 2026-08-18)
   < `recorded` (enregistrée par un club, carte vue) = `imported` (import
   Fosse) < `validated` (délivrée dans Dive Manager, futur module
   compétences). Ré-enregistrer une certification existante : autorisé si la
   nouvelle provenance est de rang ≥ l'existante (mise à jour de la ligne),
   sinon 409.
   **Décision produit 2026-08-18 : les certifications sont saisies par les
   clubs uniquement** — aucun endpoint d'auto-déclaration. La valeur
   `declared` reste dans le schéma (CHECK + rang 0) au cas où l'auto-
   déclaration rouvrirait ; rien ne l'écrit aujourd'hui.
3. Une seule ligne par (user, certification) — contrainte DB.
4. Les codes sont ceux de la lib `ffessm` (golden test) ; `PA60` n'existe
   pas ; code inconnu à l'écriture → 422. À la lecture, un code inconnu en
   base est loggé en erreur et ignoré (jamais inventé).
5. **Brevet en préparation** : un seul à la fois (comme Fosse) ; doit être
   préparable en club (`!is_externally_delivered`) et non détenu ;
   l'éligibilité `ffessm` non satisfaite n'est pas bloquante (le club
   décide) mais est remontée à l'UI.
6. Le user édite son propre profil (hors CACI, hors certifications) ; un
   club (`club.members.manage`) édite le profil de ses membres actifs,
   saisit le CACI et enregistre/supprime les certifications ; ces écritures
   club exigent un principal réel (audité). Les écritures `/me/*` sont
   refusées en impersonation (même règle que le mot de passe).
7. **Date de naissance** : requise pour toute dérivation `ffessm` ; sans
   elle, les endpoints dérivés répondent `profile_incomplete` (jamais de
   valeur devinée). Sa modification est auditée (old/new) — elle change des
   prérogatives légales.
8. **CACI** : on ne stocke que la **date de fin de validité** (aucune donnée
   médicale) ; saisie par le **club uniquement** au vu du certificat (le PUT
   self conserve la valeur stockée). Carte dédiée en tête de fiche membre.
9. Données dérivées jamais persistées (la réglementation change — seule la
   lib fait foi, snapshot versionné).

## 3. Entités & objets-valeurs

- `UserProfile` (1-1 user) — entité. Valeurs : `GearPreferences`
  (détendeur, stab + taille, nitrox, 2ᵉ détendeur), dates CACI/licence.
- `UserCertification` — entité (user × code ffessm, obtained_on?,
  provenance, recorded_by, recorded_in_club).
- `UserTrainingStatus` — entité (stagiaire MF1, valid_until).
- Valeurs réutilisées de `ffessm` : `Certification`, `TrainingStatus`,
  dérivés (`Aptitudes`, `SupervisionCapacity`, restrictions).

## 4. Schéma PostgreSQL (nouvelle migration `m20260817_000002_user_profiles`)

```
user_profiles
  user_id       uuid PK → users ON DELETE CASCADE
  birthdate     date NULL
  phone         text NULL
  licence_number text NULL
  medical_certificate_valid_until date NULL
  gear_regulator bool NOT NULL DEFAULT true
  gear_stab      bool NOT NULL DEFAULT true
  gear_stab_size text NULL CHECK (gear_stab_size IN ('XS','S','M','L','XL'))
  gear_nitrox    bool NOT NULL DEFAULT false
  gear_second_regulator bool NOT NULL DEFAULT false
  preparing_certification text NULL         -- code ffessm, validé app
  created_at, updated_at timestamptz NOT NULL

user_certifications
  id            uuid PK
  user_id       uuid NOT NULL → users ON DELETE CASCADE
  certification text NOT NULL               -- code ffessm, validé app
  obtained_on   date NULL
  provenance    text NOT NULL CHECK (provenance IN
                  ('declared','recorded','imported','validated'))
  recorded_by_user_id uuid NULL → users ON DELETE SET NULL
  recorded_in_club_id uuid NULL → clubs ON DELETE SET NULL
  created_at, updated_at timestamptz NOT NULL
  UNIQUE (user_id, certification)
  INDEX (user_id)

user_training_statuses
  id          uuid PK
  user_id     uuid NOT NULL → users ON DELETE CASCADE
  status      text NOT NULL CHECK (status IN ('stagiaire_mf1'))
  valid_until date NOT NULL
  created_at  timestamptz NOT NULL
  UNIQUE (user_id, status)
```

Pas de CHECK DB sur les codes ffessm (une seule source de vérité : la lib ;
un CHECK dupliqué dériverait). Validation à l'écriture applicative.

## 5. Interfaces (API + domaine)

**Self (`/api/v1/me/...`)** — écritures refusées en impersonation :
```
GET  /me/profile           → profil + certifications + statuts + dérivés*
PUT  /me/profile           → birthdate, phone, licence, gear, preparing
                             (le CACI stocké est conservé tel quel)
GET/PUT/DELETE /me/stamp   → tampon (PUT réservé E2+ certifié)
```
(Pas d'endpoints `/me/certifications` — saisie club uniquement,
décision 2026-08-18.)
**Club (`/api/v1/clubs/{slug}/members/{user_id}/...`)** — cible = membre
ACTIF du club, sinon 404 :
```
GET  .../profile           [club.members.view]    → idem self + provenances
PUT  .../profile           [club.members.manage]  (audité)
POST .../certifications    [club.members.manage]  → recorded (audité)
PUT  .../training-statuses [club.members.manage]  → stagiaire MF1 (audité)
DELETE .../certifications/{id} [require_real + members.manage] (audité)
```
**Enrichissement** de `GET /clubs/{slug}/memberships` : chaque ligne gagne
`diver_rank_label`, `supervision`, `has_rifap`, `medical_expired` —
certifications de la page chargées en **une** requête (`user_id IN (…)`).

\* dérivés = `ffessm::exercisable_aptitudes` + restrictions +
`supervision` + `eligible_next` + `REGULATION_SNAPSHOT` ; ou
`profile_incomplete: true` sans date de naissance.

**Domaine (Rust)** : `users::services::profiles` (lecture/écriture profil,
upsert certification avec règle de provenance, suppression) ;
`users::services::derived` (assemble `ffessm::Person` depuis les lignes DB
— seul endroit qui parse les codes). Handlers minces comme d'habitude.

## 6. Autorisations & isolation tenant

| Action | Qui |
|---|---|
| Lire son profil | tout utilisateur authentifié (impersonation incluse : « voir comme ») |
| Éditer son profil (hors CACI), gérer son tampon | utilisateur authentifié **non impersoné** (403 sinon — données user-globales, escalade sinon via `club.impersonate`) |
| Lire le profil d'un membre | adhésion active au club + `club.members.view` |
| Éditer profil membre, saisir le CACI, enregistrer/supprimer une cert, statut stagiaire, tampon | `require_real` + `club.members.manage` |

**Isolation — LA nouvelle surface IDOR** : `user_id` est global ; le
handler club vérifie que la cible a une adhésion **active** dans le club du
tenant (requête scoping), sinon 404. Testé cross-club obligatoirement.
Adhésion suspendue ⇒ profil invisible pour ce club.

## 7. Dépendances, erreurs, cas limites

- Dépend de : `identity` (users, audit), `tenancy` (Tenant, memberships),
  `ffessm`. Rien ne dépend de `users` pour l'instant.
- Erreurs : `AppError` existants (422 code inconnu / preparing invalide /
  taille stab, 409 provenance insuffisante ou doublon, 404 cible hors club).
- Cas limites : profil sans birthdate (dérivés → incomplete) ;
  `obtained_on` dans le futur → 422 ; deux clubs enregistrent la même cert
  → une ligne, provenance/date mises à jour ; club supprimé →
  `recorded_in_club` NULL (cert conservée) ; user désactivé → profil lisible
  par ses clubs (historique) mais plus modifiable par lui ; mineurs → les
  restrictions arrivent par la lib, l'UI les affiche.

## 8. Performance

GET profil = 3 requêtes point (profil, certs, statuts) + calcul pur.
Liste membres enrichie = +2 requêtes par page (profils IN, certs IN) —
jamais de N+1. Payload : dérivés compacts (codes + libellés), pas de tables
réglementaires dans les réponses.

## 9. Stratégie de tests

- **Isolation/IDOR (prioritaire)** : club A lit/édite le profil d'un membre
  du club B → 404 ; membre suspendu → 404 ; user sans club lisible par
  personne (sauf lui et admin global).
- Autorisations : chaque route × (membre simple, members.view,
  members.manage, admin global non membre, impersonation — écritures /me
  refusées, routes /me/certifications inexistantes).
- Provenance : recorded→validated ok, validated→recorded 409, admin peut
  supprimer une validated, unique (user, cert).
- Dérivés : persona complet via HTTP (certs en base → aptitudes/grade/
  eligible_next corrects, `snapshot` présent) ; profil sans birthdate →
  incomplete ; code inconnu injecté en base → ignoré + loggé.
- Enrichissement liste : exactitude + comptage de requêtes stable.
- Frontend : écran « Mon profil plongée » et fiche membre (vitest sur les
  mappings ; visuel validé par toi).

## 10. Migration Fosse

- `people` → users `invited` (email normalisé) + `user_profiles`
  (phone, gear defaults mappés).
- `diving_level` (blob CSV) → `user_certifications` provenance `imported` :
  `N1..N5, PE40, PA20, PA40, PE60, PESH6, PESH12` → codes directs ;
  `E1`→INITIATEUR ; `E2`→INITIATEUR+N4 ; `E3`→MF1 ; `E4`→MF2 ;
  `PA60`→N3 ; `preparing_X` → `preparing_certification` (PA60→N3).
  **Token inconnu = échec bruyant de l'import** (rapport), jamais ignoré.
- ⚠️ **Fosse n'a pas de date de naissance** : profils importés sans
  birthdate → dérivés indisponibles jusqu'à saisie (par le membre à sa
  première connexion, ou en masse par l'admin). À trancher (question 1).

## 11. Plan d'implémentation

1. Migration + entités + services (provenance, dérivés via ffessm).
2. Endpoints self, puis club (+ enrichissement liste membres).
3. Tests intégration (isolation d'abord) — backend complet avant frontend.
4. Frontend : « Mon profil plongée » (self) + fiche membre + badges liste.
5. Docs (MODULES.md : `members` devient `users`, DOMAIN.md ajusté).

## 12. Questions ouvertes — TRANCHÉES (product owner, 2026-08-17)

1. **Birthdate** : saisie par le user dans son profil ; un **bandeau rouge
   « profil incomplet »** (avec lien vers le profil) s'affiche tant que la
   date de naissance manque.
2. Suppression d'une certification `validated` : **admin de club (réel) ET
   admin global** — comme les autres non-declared.
3. **CACI et RIFAP saisis par les admins du club** (par user via la fiche
   membre, ou en masse par CSV — le volet CSV rejoint le module `imports`).
4. `licence_number` dans le profil : confirmé. **Ajouts** : `instructor
   number` (numéro d'encadrant) et **tampon** image (PNG/JPEG ≤ 512 Kio,
   table dédiée `user_stamps`, magic-bytes vérifiés, upload réservé aux
   détenteurs d'un grade certifié ≥ E2) — servira aux documents de niveau.
