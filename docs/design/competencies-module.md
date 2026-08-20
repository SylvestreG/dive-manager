# Module `competencies` — design

Status: **approuvé (PO « go », 2026-08-19) — implémenté le 2026-08-20,
revue senior passée + correctifs appliqués le 2026-08-20** ; décisions
consolidées dans ADR-0008.

Amendement post-revue (PO, 2026-08-20) : **le global admin réel peut valider
« au nom d'un encadrant »** — champ optionnel `validated_by_user_id` sur
l'écriture de validation, réservé au global admin réel ; l'encadrant désigné
doit remplir exactement les conditions d'un validateur direct (membre actif
du club, grade dérivé suffisant, ≠ plongeur). L'état courant (et le PDF)
portent l'encadrant désigné ; l'événement d'historique et l'audit gardent
l'admin comme acteur (`on_behalf_of` dans l'audit).

Décisions produit fondatrices (product owner, 2026-08-19) :

1. Les compétences sont une **fondation de la plateforme, pas un module
   club**. L'arbre de compétences (niveau → domaine → module → acquis) est
   **global**, géré par le global admin uniquement — comme le référentiel
   FFESSM. Les validations d'acquis **suivent l'utilisateur** quel que soit le
   club (user-global), dans la même logique que la décision 2026-08-17 sur le
   profil de plongée. Ceci **remplace** le design per-club annoncé par
   `MODULES.md` et `DOMAIN.md` (mise à jour de ces documents à
   l'implémentation).
2. Les **étapes de validation sont fixes, codées en dur** — pas de table, pas
   d'éditeur : exactement trois étapes, `Vu → À perfectionner → Validé`. Pour
   le N1 (et en dessous), « Validé » peut s'obtenir en tout milieu ; au-dessus
   du N1, « Validé » signifie **forcément en mer** (règle de terrain +
   libellé, pas une étape supplémentaire).
3. Le **grade de validateur requis dépend du niveau préparé**, en dur — plus
   de grade minimal par acquis : E1 valide < N2, E2 valide < N3, E3 valide
   < N4, E4 au-delà.

Conséquence structurelle : aucune table de ce module ne porte de `club_id` en
propriété. Le club apparaît uniquement comme **contexte d'attribution** (où la
validation a été faite) et comme **frontière d'accès** (qui a le droit de
valider qui, qui voit quoi).

---

## 1. Responsabilité du module

Le module possède :

- le **référentiel global de compétences** : arbre `certification (ffessm) →
  domaine → module → acquis` ;
- les **règles fixes** : les trois étapes de validation (enum Rust, pas de
  donnée) et le grade de validateur requis par niveau ;
- l'**état de validation par (utilisateur, acquis)** + un **historique
  append-only** de tous les événements de validation (création, avancement,
  retour arrière, suppression) — ce que Fosse prétend avoir et n'a pas ;
- les **documents de niveau** : un template PDF global par certification, les
  positions de tampon par acquis, et la génération du PDF rempli pour un
  utilisateur ;
- le calcul de **progression** (par module/domaine/niveau, pondéré).

**Hors périmètre** :

- toute gestion d'étapes (table, CRUD, éditeur, couleurs/icônes
  configurables) et tout grade minimal par acquis — supprimés par les
  décisions n° 2 et 3 ci-dessus ;
- le constructeur de groupes de sortie (Fosse `SortieGroupesPage`) — c'est une
  vue `competencies` × `activities`, elle attend le module `activities` ;
- les tableaux de bord statistiques de Fosse (implémentation cliente cassée :
  `slice(0,20)`, deux formules de % divergentes) — plus tard, en agrégats
  serveur ;
- la **délivrance automatique de la certification** quand tous les acquis sont
  validés — hors v1 (PO 2026-08-19, Q3) : la délivrance reste un acte manuel
  du module `users` (`provenance = validated`) ; un indicateur « éligible »
  pourra venir plus tard ;
- l'incrustation de l'**image du tampon** d'encadrant sur le PDF — hors v1
  (PO 2026-08-19, Q4) : texte seul, parité Fosse ; l'image viendra dans une
  itération suivante ;
- la cascade de niveaux (« valider N2 implique N1 ») — déjà couverte par
  `users`/`ffessm` (`covered_by`, arbre de certifications) ;
- le modèle plat legacy de Fosse (table `competencies`, ancienne v1 sans
  étapes ni validations, page orpheline) — abandonné, non migré (vérification
  d'innocuité sur données réelles au gel de l'importeur) ;
- l'écriture de `preparing_certification` — reste au module `users`.

---

## 2. Analyse Fosse (synthèse)

Détails : `docs/analysis/fosse-backend.md` §1.9-1.10, §2.6-2.7,
`functional-inventory.md` §11-12. Rappel des faits CONFIRMÉS qui pilotent ce
design :

- **Deux modèles** : un modèle plat mort (`competencies`, routes CRUD sans
  aucune protection) et le modèle hiérarchique vivant.
- **Aucun historique** : `skill_validations` est unique `(person, skill)` et
  muté sur place ; la page « logs » est un SELECT de l'état courant.
- **Règle monotone** (pas de retour arrière sauf admin réel non-impersonné)
  présente… mais contournable par `PUT /skill-validations/:id` qui ne vérifie
  ni l'ordre ni le grade du validateur.
- **Gate min-validateur fail-open** : token inconnu ⇒ check silencieusement
  sauté ; et l'échelle scalaire mélange filières (N4=40 ≥ E1=35 ⇒ un N4
  non-encadrant peut valider un acquis « min E1 »). Le grade requis était par
  acquis (`min_validator_level`, défaut E2, valeurs vues : E1..E4, N3, N4) —
  remplacé ici par la règle fixe par niveau.
- **`/level-documents/*` : zéro contrôle serveur** — n'importe quel utilisateur
  authentifié peut remplacer un template ou générer le PDF d'autrui.
- `GET /skill-validations` non filtré : fuite de toutes les validations à tout
  authentifié (« handled by the frontend »).
- Étapes Fosse (table + éditeur) : `VU_PISCINE → A_PERFECTIONNER →
  ACQUIS_PISCINE → A_RETRAVAILLER_MER → VALIDE_MER (final)` ; l'ordre 3/4
  contredisait la règle monotone. La décision « trois étapes en dur » supprime
  toute cette surface (et ses bugs : sort_order mutable qui requalifie
  rétroactivement le « validé », multiples étapes finales, RESTRICT → 500).
- Positions de tampon : rectangle en points PDF (origine bas-gauche, ancre
  bas-gauche), unique `(skill, level)`, `font_size` figé à 8 côté UI ; PDF
  généré : annotations (effaçables), seules les étapes finales sont tamponnées,
  date + nom du validateur.
- Éditeur de tampons frontend (pdfjs-dist) : dessiner/déplacer/redimensionner,
  zoom 50-200 %, persistance immédiate — la pièce d'UX distinctive à préserver.
- Aucune transaction, N+1 partout, `validated_at` futur accepté, aucune
  pagination des logs, aucun test.
- Le lien « niveau préparé ↔ arbre » est purement frontend : le client passe
  `?diving_level=` ; le serveur ne connaît pas la préparation.

À préserver (charte `DOMAIN.md` §5) : arbre de compétences ; éditeur de
positions de tampon ; règle forward-only avec étapes désactivées visibles ;
modale de confirmation avec formulation d'irréversibilité ; légende d'étapes ;
états vides parlants côté membre ; nommage
`Competences_{niveau}_{NOM}_{Prénom}.pdf`.

---

## 3. Règles métier

1. **Arbre global** (PO 2026-08-19). Un seul référentiel pour toute la
   plateforme, indexé par code de certification `ffessm`
   (`Certification::code()`, validé à l'écriture applicative — token inconnu
   → 422). Géré par le global admin uniquement.
2. **Étapes fixes** (PO 2026-08-19), enum Rust `ValidationStage` :
   `vu (1) → a_perfectionner (2) → valide (3)`. `valide` est l'étape finale
   partout. Libellé/règle de milieu selon le niveau du domaine :
   - N1 et en dessous (`diver_rank() ≤ 10`, PESH inclus) : « Validé » — tout
     milieu ;
   - au-dessus du N1 (`diver_rank() > 10`) : « Validé en mer » — la
     validation finale se fait forcément en mer (libellé affiché partout,
     rappel dans la confirmation de validation).
   Pas de table, pas de CRUD, pas d'éditeur ; libellés/couleurs en dur côté
   frontend.
3. **Grade de validateur requis par niveau** (PO 2026-08-19), fonction fixe
   `required_validator_grade(certification)` :
   - niveau < N2 (N1, PESH6, PESH12, niveaux sans rang plongeur) → **E1** ;
   - N2 (et aptitudes de rang N2 ≤ rang < N3) → **E2** ;
   - N3 (rang < N4) → **E3** ;
   - N4 et au-delà → **E4**.
   Per ADR-0007 le droit de valider découle du **grade d'enseignement dérivé**
   (`ffessm::supervision(...).teaching_certified`, certifications uniquement —
   le statut stagiaire MF1 ne donne **rien**, ADR-0007 Q5), jamais d'un niveau
   plongeur ni d'une permission.
4. **Qui peut valider** (PO 2026-08-19) : un utilisateur avec
   `teaching_certified >= required_validator_grade(niveau)`, **membre actif
   d'un club où le plongeur est aussi membre actif**. La validation est
   ensuite visible partout (user-global).
5. **Progression monotone** : une étape ne peut qu'avancer (ou être
   re-tamponnée à l'identique : met à jour date/validateur/notes, événement
   journalisé). Régresser ou supprimer = **global admin réel** (non-impersoné)
   uniquement (PO 2026-08-19) → sinon **409**. Contrairement à Fosse, il n'y a
   qu'UN chemin d'écriture — pas de `PUT` de contournement. L'ordre des étapes
   étant en dur, il ne peut plus être requalifié après coup.
6. **Historique append-only** : chaque écriture (validated / reverted /
   deleted) produit un événement immuable, dénormalisé (nom d'acquis, étape,
   niveau) pour survivre aux restructurations de l'arbre. Aucun événement
   n'est jamais modifié ni supprimé.
7. **Pas d'auto-validation** : `validator_user_id != user_id` → sinon 422.
   (Fosse ne le bloquait pas ; aucun cas d'usage légitime.)
8. **`validated_at`** : date (jour), défaut = aujourd'hui, **futur refusé**
   (422) ; l'antidatage est permis (réalité terrain : saisie après la fosse).
9. **Écritures refusées en impersonation** : toutes les écritures du module
   (validations comme gestion d'arbre) exigent un principal réel — cohérent
   avec le module `users` (« /me writes refused while impersonating »). Fosse
   attribuait la validation à l'identité impersonée sans avertissement : non
   reproduit.
10. **Visibilité club** : un membre du club (staff ou encadrant certifié) voit
    les compétences d'un plongeur **seulement** si celui-ci est membre actif
    du club (sinon 404, précédent `ensure_member`). L'utilisateur voit toujours
    les siennes (`/me`).
11. **PDF rempli** : seules les validations à l'étape `valide` sont
    tamponnées ; contenu = date `jj/mm/aaaa` + prénom nom du validateur
    (+ grade d'enseignement dérivé s'il tient sur deux lignes). Texte **aplati
    dans le flux de contenu** (non effaçable), pas une annotation (défaut
    Fosse).
12. **Pourcentage unique** : `validés / total_acquis`, pondéré, calculé
    serveur (Fosse avait deux formules divergentes ; celle-ci est la
    correcte).

---

## 4. Entités & objets-valeurs

Entités : `CompetencyDomain`, `CompetencyModule`, `CompetencySkill`,
`SkillValidation` (état courant), `SkillValidationEvent` (historique),
`LevelDocument`, `LevelDocumentPosition`.

Objets-valeurs : `ValidationStage` (enum en dur du module : `vu`,
`a_perfectionner`, `valide`, avec `rank()` et le libellé milieu par niveau) ;
`required_validator_grade(Certification) -> SupervisionGrade` (fonction fixe,
règle n° 3) ; réutilisés du crate `ffessm` : `Certification` (le « niveau » de
l'arbre et des documents, `diver_rank()` pour milieu et grade requis) et
`SupervisionGrade`.

États d'une validation : `absent → (vu|a_perfectionner|valide)* → deleted`,
transitions croissantes seulement (sauf global admin réel), chaque transition
étant un événement.

---

## 5. Schéma PostgreSQL (migration `m20260819_000003_competencies`)

Conventions du projet : uuid v7 applicatif, `timestamptz`, texte + CHECK pour
les petits ensembles fermés, **pas de CHECK sur les codes ffessm** (source de
vérité = crate), index explicites sur chaque FK, DSL SeaORM.

```
competency_domains
  id uuid PK
  certification text NOT NULL          -- code ffessm, validé applicativement
  name text NOT NULL CHECK (name <> '')
  sort_order integer NOT NULL
  created_at / updated_at timestamptz NOT NULL
  UNIQUE (certification, name)         -- uq_competency_domains_cert_name
  INDEX idx_competency_domains_certification

competency_modules
  id uuid PK
  domain_id uuid NOT NULL FK → competency_domains ON DELETE CASCADE
  name text NOT NULL CHECK (name <> '')
  sort_order integer NOT NULL
  created_at / updated_at
  UNIQUE (domain_id, name)
  INDEX idx_competency_modules_domain_id

competency_skills
  id uuid PK
  module_id uuid NOT NULL FK → competency_modules ON DELETE CASCADE
  name text NOT NULL CHECK (name <> '')
  description text NULL
  sort_order integer NOT NULL
  created_at / updated_at
  UNIQUE (module_id, name)
  INDEX idx_competency_skills_module_id

user_skill_validations                  -- état courant, 1 ligne / (user, acquis)
  id uuid PK
  user_id uuid NOT NULL FK → users ON DELETE CASCADE
  skill_id uuid NOT NULL FK → competency_skills ON DELETE CASCADE
  stage text NOT NULL CHECK (stage IN ('vu','a_perfectionner','valide'))
  validated_at date NOT NULL
  validated_by_user_id uuid NULL FK → users ON DELETE SET NULL   -- attribution
  validated_in_club_id uuid NULL FK → clubs ON DELETE SET NULL   -- attribution
  notes text NULL
  created_at / updated_at
  UNIQUE (user_id, skill_id)           -- uq_user_skill_validations_user_skill
  INDEX idx_user_skill_validations_skill_id
  INDEX idx_user_skill_validations_validated_by / _validated_in_club

user_skill_validation_events            -- append-only, jamais modifié
  id uuid PK
  user_id uuid NOT NULL FK → users ON DELETE CASCADE
  skill_id uuid NULL FK → competency_skills ON DELETE SET NULL
  skill_name text NOT NULL              -- dénormalisé (survit à l'arbre)
  certification text NOT NULL           -- dénormalisé
  action text NOT NULL CHECK (action IN ('validated','reverted','deleted'))
  stage text NULL CHECK (stage IN ('vu','a_perfectionner','valide'))
                                        -- étape cible (NULL pour deleted)
  previous_stage text NULL CHECK (…idem…)
  validated_at date NULL
  performed_by_user_id uuid NULL FK → users ON DELETE SET NULL
  performed_in_club_id uuid NULL FK → clubs ON DELETE SET NULL
  notes text NULL
  created_at timestamptz NOT NULL
  INDEX idx_usv_events_user_id (+ created_at pour la pagination)
  INDEX idx_usv_events_skill_id / _performed_by / _performed_in_club

level_documents                          -- 1 template global / certification
  id uuid PK
  certification text NOT NULL UNIQUE (uq_level_documents_certification)
  file_name text NOT NULL
  file_data bytea NOT NULL               -- précédent user_stamps : blob en DB
  page_count integer NOT NULL
  created_at / updated_at

level_document_positions
  id uuid PK
  level_document_id uuid NOT NULL FK → level_documents ON DELETE CASCADE
  skill_id uuid NOT NULL FK → competency_skills ON DELETE CASCADE
  page integer NOT NULL CHECK (page >= 1)
  x / y / width / height real NOT NULL   -- points PDF, origine bas-gauche,
                                          -- ancre bas-gauche (modèle Fosse)
  font_size real NOT NULL DEFAULT 8.0
  created_at / updated_at
  UNIQUE (level_document_id, skill_id)
  INDEX idx_level_document_positions_skill_id
```

Justifications :

- **Pas de table d'étapes, pas de grade par acquis** (décisions PO) : `stage`
  est un texte contraint par CHECK (ensemble fermé, stable) + enum Rust ; le
  grade requis est une fonction pure du niveau (connaissance ffessm → pas en
  base, convention « pas de CHECK sur la connaissance ffessm »).
- **FK réelle positions → document** (uuid, CASCADE) : corrige l'orphelinat
  Fosse (chaîne `level` sans FK). Re-upload = update des bytes du même
  document ⇒ positions conservées ; pages devenues hors-borne signalées par
  l'éditeur, ignorées à la génération (cas limite §7).
- Pas de CHECK sur `certification` (convention ffessm) ; CHECK sur `stage` et
  `action` (ensembles fermés stables).
- Volume attendu : arbre ≤ quelques centaines d'acquis ; validations ≈
  membres × acquis du niveau préparé (USI : ~10⁴ lignes max) ; événements ≈
  2-3× validations. Aucun index exotique nécessaire.
- **Aucun seed** : l'arbre se construit via l'API (ou l'importeur Fosse).

---

## 6. Interfaces (API + domaine)

### Lecture globale (tout utilisateur authentifié)

- `GET /competencies/levels` — certifications ayant un arbre et/ou un
  template (`certification, domains_count, has_document, page_count,
  required_validator_grade, sea_validation: bool`) — sans blob.
- `GET /competencies/tree?certification=X` — domaines→modules→acquis + règles
  du niveau (grade requis, milieu) (borné, non paginé — précédent
  `/catalog/certifications`).
- `GET /me/competencies?certification=X` — arbre + mes validations +
  progression ; défaut : `preparing_certification` de mon profil, 422 si aucun
  des deux.
- `GET /me/competencies/document?certification=X` — mon PDF rempli
  (`Content-Type: application/pdf`, `Cache-Control: private, no-store`).

### Administration du référentiel (global admin ; suppressions : **réel**)

- `POST/PUT/DELETE /competencies/domains[/{id}]`
- `POST/PUT/DELETE /competencies/modules[/{id}]`
- `POST/PUT/DELETE /competencies/skills[/{id}]`
- La suppression d'un domaine/module/acquis **détruit en cascade des
  validations user-globales** : réponse de pré-vol `GET
  /competencies/{kind}/{id}/impact` (`validations_count`) pour que l'UI
  affiche un vrai avertissement (Fosse : `window.confirm` muet).
- `PUT /competencies/levels/{certification}/document` — upload JSON base64
  (précédent tampon) ; validation : magic bytes `%PDF`, parse lopdf,
  `MAX_LEVEL_DOCUMENT_BYTES = 5 MiB` (limite de body relevée sur cette route).
- `DELETE /competencies/levels/{certification}/document` `[réel]`
- `GET /competencies/levels/{certification}/document` — téléchargement (admin
  + éditeur).
- `GET/PUT/DELETE /competencies/levels/{certification}/positions[...]` —
  upsert par acquis + batch (transactionnel) pour l'éditeur.
- `GET /competencies/validation-events?user_id=&page=` — historique paginé
  `[global admin]`.

### Contexte club (`/clubs/{slug}/…`, extracteur `Tenant`)

- `GET /clubs/{slug}/members/{user_id}/competencies?certification=X`
  `[club.members.view OU encadrant certifié — précédent require_members_view]`
  — arbre + validations + progression du membre (404 si non membre actif).
- `PUT /clubs/{slug}/members/{user_id}/skill-validations/{skill_id}`
  `{stage, validated_at?, notes?}` `[encadrant certifié ≥
  required_validator_grade(niveau) ; réel ; audité]` — upsert transactionnel :
  gate grade → gate monotone (régression : global admin réel sinon 409) →
  écriture état courant + événement + `audit::record` club.
- `DELETE /clubs/{slug}/members/{user_id}/skill-validations/{skill_id}`
  `[global admin réel ; audité]` — supprime l'état courant, événement
  `deleted` conservé.
- `GET /clubs/{slug}/members/{user_id}/competencies/document?certification=X`
  `[club.members.view OU encadrant certifié]` — PDF rempli du membre (comble
  le manque Fosse : aucun bouton staff n'existait).
- `GET /clubs/{slug}/competencies/students?certification=X&page=` `[idem
  lecture]` — membres actifs préparant X + résumé de progression (paginé,
  requêtes batchées).

### Surface Rust interne

`competencies::domain` (enum `ValidationStage`,
`required_validator_grade(cert)`, `sea_validation(cert)`) +
`competencies::services::{tree, validations, documents, progress}` —
génériques sur `ConnectionTrait` (transactions). Gate validateur :
`users::handlers::ffessm_certifications(conn, validator_id)` →
`ffessm::supervision(...).teaching_certified`. Vérif membre actif :
`tenancy::services::memberships::find_active`. Aucune entité d'un autre module
touchée directement. Le module n'expose pour l'instant aucune API aux autres
modules (le futur builder de groupes consommera `progress`). Handlers minces
comme d'habitude.

---

## 7. Autorisations, multi-tenancy, erreurs, cas limites

| Action | Qui |
|---|---|
| Lire l'arbre / la liste des niveaux | tout utilisateur authentifié |
| Lire ses propres compétences + son PDF | soi-même (impersonation : lecture OK) |
| Lire les compétences d'un membre du club | `club.members.view` OU encadrant certifié membre actif (404 si cible non membre) |
| Valider / re-tamponner / avancer une étape | encadrant certifié `teaching_certified ≥ required_validator_grade(niveau)`, membre actif du club, cible membre actif, **réel** |
| Régresser une étape | global admin **réel** |
| Supprimer une validation | global admin **réel** |
| CRUD arbre / documents / positions | global admin (suppressions : **réel**) |
| Historique global des événements | global admin |
| Non authentifié | rien |

**La nouvelle surface IDOR** : les routes club prennent `user_id` — le test
obligatoire est « le club B ne peut ni lire ni valider un membre du club A »
(404 sur lecture ET écriture), y compris pour un encadrant E4 du club B. Et la
réciproque multi-clubs : un utilisateur membre des clubs A et B validé en A
est visible en B (donnée user-globale) — testé explicitement.

Erreurs (`AppError`) : `Validation` (certification/skill/étape inconnus, PDF
invalide, taille > 5 MiB, date future, auto-validation, base64 invalide) ;
`Forbidden` (« validating this level requires a certified E{n} instructor »,
non-membre validateur, impersonation sur écriture, non-global-admin) ;
`Conflict` (régression d'étape sans être global admin réel, doublons de nom) ;
`NotFound` (« member not found », arbre/niveau/document absents). Messages en
anglais minuscule, ajoutés à `KNOWN_MESSAGES` côté frontend.

Cas limites : niveau sans arbre → `domains: []`, progression vide (pas 404) ;
membre sans `preparing_certification` → 422 sur `/me/competencies` sans
paramètre (l'UI montre l'état vide) ; re-upload d'un template avec moins de
pages → positions hors-borne listées par l'éditeur (« hors page ») et ignorées
à la génération ; déplacement d'un domaine vers un autre niveau (PUT
`certification`) → change le grade requis et le milieu pour les validations
futures, l'existant reste valide (le gate est recalculé à chaque écriture,
jamais de droit stocké) ; re-tampon à l'identique → jamais une régression ;
validateur dont le grade dérive de certifications supprimées entre-temps → le
gate est recalculé à chaque écriture.

---

## 8. Performance

- Arbre + validations + progression d'une personne : **5 requêtes constantes**
  (domains, modules `IN`, skills `IN`, validations `IN`, validateurs `IN`)
  quel que soit la taille de l'arbre — jamais de N+1 (Fosse : 1 + D + M
  requêtes). Étapes et grade requis : zéro requête (en dur). Agrégation en
  Rust.
- Liste « élèves » : pagination memberships (2 requêtes + count) + 2 requêtes
  batchées (profils pour `preparing_certification`, comptes validés par user
  via `GROUP BY`) — pattern `page_with_users`.
- Historique : paginé (`PageParams`, max 200), index `(user_id, created_at)`.
- Blobs jamais chargés sur les chemins chauds : listes → `select_only` sans
  `file_data` (précédent `has_stamp`) ; bytes uniquement sur
  download/génération.
- Génération PDF : synchrone dans la requête (template ≤ 5 MiB, quelques
  dizaines de tampons — négligeable sur le VPS) ; pas de file d'attente.
- Écritures multi-lignes (validation + événement + audit ; batch positions) :
  **transactionnelles** (Fosse : zéro transaction).

---

## 9. Stratégie de tests

1. **Isolation tenant / IDOR (prioritaire)** : club B → membre du club A :
   lecture 404, validation 404, PDF 404 ; membre suspendu → 404 ; user
   multi-clubs : validé en A, visible en B ; encadrant E4 sans club commun →
   404.
2. **Matrice d'autorisation** : membre simple ne valide pas (403) ; matrice
   grade × niveau (E1 sur arbre N1 → 200, E1 sur N2 → 403, E2 sur N2 → 200,
   E2 sur N3 → 403, E3 sur N3 → 200, E3 sur N4 → 403) ; stagiaire MF1 → 403
   (rien) ; écritures en impersonation → 403 ; régression par encadrant → 409,
   par club admin → 409, par global admin impersonné → 409, par global admin
   réel → 200 ; CRUD arbre par club admin → 403 ; routes documents par membre
   → 403 (le trou Fosse) ; sweep fail-closed des nouvelles routes.
3. **Règles métier** : matrice monotone (avance / re-tampon / saut direct à
   `valide` / régression) ; événement append-only émis pour chaque écriture
   (y compris deleted) et jamais réécrit ; auto-validation 422 ; date future
   422 ; unicité `(user, skill)` sous concurrence (upsert transactionnel) ;
   progression pondérée (module/domaine/niveau, total 0 → 0 %) ; unités
   `required_validator_grade` et `sea_validation` sur tout le catalogue
   ffessm.
4. **Base de données** : contraintes (uniques, CHECK stage/action, cascades
   domaine→…→validations, SET NULL événements après suppression d'acquis —
   l'historique dénormalisé survit).
5. **Documents** : upload non-PDF → 422, > 5 MiB → 422 ; génération : seules
   les validations `valide` tamponnées, nom de fichier, positions hors-page
   ignorées ; positions batch transactionnel.
6. **Frontend** : composants progression/légende d'étapes (vitest), formule de
   % unique, états vides membre, libellé « Validé » vs « Validé en mer ».

---

## 10. Migration Fosse

Étendue par l'importeur (module dédié, hors de ce périmètre d'implémentation,
mais le mapping est fixé ici) :

- `competency_domains.diving_level` → `certification` via
  `Certification::parse` — **token inconnu = échec bruyant**.
- `competency_skills.min_validator_level` : **colonne abandonnée** (le grade
  requis est désormais fonction du niveau, PO 2026-08-19). Le rapport de
  réconciliation signalera les acquis Fosse dont le `min_validator_level`
  divergeait de la règle fixe, pour information.
- Étapes Fosse → enum fixe (mapping, à confirmer sur données réelles) :
  `VU_PISCINE → vu` ; `A_PERFECTIONNER → a_perfectionner` ;
  `A_RETRAVAILLER_MER → a_perfectionner` ; `ACQUIS_PISCINE → valide` si
  niveau ≤ N1, sinon `a_perfectionner` (au-dessus du N1, « validé » = en mer) ;
  `VALIDE_MER → valide`. Code d'étape inconnu = échec bruyant.
- `skill_validations` → une ligne `user_skill_validations` (personne → user
  via le mapping identité de l'importeur) + **un** événement `validated`
  (l'histoire antérieure n'existe pas dans Fosse, rien à inventer) ;
  `validated_in_club_id = USI`.
- `level_documents` + `skill_document_positions` → copies globales (FK réelle
  vers le document).
- Table plate `competencies` (ancienne v1) : **non migrée** — vérifier qu'elle
  est vide ou sans valeur en production avant gel de l'importeur.

---

## 11. Plan d'implémentation

Backend complet avant frontend, tests avec chaque tranche :

1. Migration `m20260819_000003_competencies` + entités + `competencies::domain`
   (enum `ValidationStage`, `required_validator_grade`, `sea_validation`
   + tests unitaires).
2. Services + handlers lecture globale (levels, tree) + CRUD référentiel
   global admin (+ endpoint impact) + tests autorisation/CRUD.
3. Services validations : gate grade par niveau, monotone, événements,
   transactions ; routes club (PUT/DELETE/GET) + `/me/competencies` ; audit
   club ; tests IDOR/matrice/monotone (le gros morceau).
4. Progression + liste élèves paginée + historique paginé + tests perf/N+1.
5. Documents de niveau : upload/download/positions (batch transactionnel) +
   génération lopdf (texte aplati) + tests.
6. Frontend membre : « Mes compétences » (arbre, trois étapes, progression,
   états vides, bouton PDF) — périmètre store v1.
7. Frontend club : onglet Compétences (élèves préparant, fiche de validation
   avec boutons d'étapes, confirmation avec rappel « en mer » au-dessus du N1,
   désactivation visible des étapes interdites, re-tampon) ; dépose de
   l'onglet « Modules » (Q6 — le registre serveur reste intact).
8. Frontend platform admin : gestion arbre (avec avertissements d'impact),
   documents + éditeur de positions (pdfjs-dist, desktop).
9. Documentation : MODULES.md (statut + nouveau modèle global), DOMAIN.md,
   DECISIONS.md (ADR du pivot global + étapes fixes + grade par niveau),
   ROADMAP.md, TODO.md.

---

## 12. Questions ouvertes — TRANCHÉES (product owner, 2026-08-19)

- **Q1 — Grade minimal de validateur** : ~~mapping N3/N4~~ → remplacé par la
  règle fixe par niveau : **E1 < N2, E2 < N3, E3 < N4, E4 au-delà**. Plus de
  grade par acquis.
- **Q2 — Étapes** : **trois étapes seulement** (vu / à perfectionner /
  validé) ; N1 : tout milieu ; au-dessus : validation en mer forcément
  (libellé « Validé en mer »).
- **Q3 — Délivrance de la certification** : **hors v1** (validé PO) — acte
  manuel via `users` ; indicateur « éligible » plus tard.
- **Q4 — Tampon image sur le PDF** : **texte seul en v1** (validé PO) ; image
  du tampon en itération suivante.
- **Q5 — Table plate legacy** : **non migrée** (validé PO) — ancienne v1 des
  compétences Fosse (liste niveau/nom sans validations), page orpheline ;
  l'importeur vérifiera sur les données de production qu'elle ne contient rien
  de précieux.
- **Q6 — Système de modules par club** : **retiré de l'UX** (PO 2026-08-19 :
  « pour l'instant on ne fait que du core/fondation »). Même traitement que
  l'onglet Rôles (ADR-0007 follow-up) : l'onglet club « Modules » et son
  interrupteur disparaissent du frontend ; le registre `enabled_modules` de
  `tenancy` (tables, `PUT /clubs/{slug}/modules`, tests) reste server-side,
  intact et prêt si un écran revient. Le module `competencies` ne consulte
  jamais ce registre ; ses accès sont gouvernés uniquement par les règles §7.
  La dépose de l'onglet est incluse dans la tranche frontend de ce module.
