# Lib métier `ffessm` — design

Status: **implemented** (2026-08-17, `backend/ffessm`, 40 tests). Basé sur
`docs/analysis/ffessm-levels.md` (MFT en vigueur + Code du Sport 2026).
Périmètre validé par le product owner : v1 complète (jeunes, trimix,
diplômes d'État), statut stagiaire MF1, RIFAP/TIV/permis trackés, contraintes
cliniques Handisub en warnings.

Écarts/décisions d'implémentation :
- L'autorisation parentale des mineurs est une règle GLOBALE de
  `check_eligibility` (MFT généralités), pas une ligne par brevet.
- `SupervisionCapacity` distingue `teaching` (statuts inclus) de
  `teaching_certified` : le stagiaire pédagogique MF1 enseigne (E2) mais
  n'est JAMAIS directeur de plongée (MFT MF1 juil. 2026).
- Brevets jeunes bornés 8-14 ans (`MaxAge`), Plongeur d'Or : espace 0-12 m
  avant 12 ans puis 0-20 m.
- La monotonie au vieillissement se vérifie par couverture (`covers`) : l'Or
  passe de PE12 à PE20 à 12 ans.

Revue senior du 2026-08-17 — corrections appliquées (46 tests) :
- **Enseignement** : les élèves plongent au-delà de leur aptitude détenue par
  définition d'une formation (A322-83..87) — l'espace est borné par le grade
  de l'encadrant ; dépassement remonté en warning `TrainingBeyondHeldAptitude`.
- Les restrictions propres à l'autonomie ne polluent plus les palanquées
  encadrées ; `Aptitudes`/`SCUBA_MIN_AGE` re-exportés ; titres d'État exclus
  de `eligible_next` ; contrat serde gelé par un golden test des 37 codes +
  round-trip JSON ; `Display` des grades ; tests MaxAge/TeachingGrade.

## 1. Objectif et périmètre

Une **crate Rust pure** (`backend/ffessm`, membre du workspace) qui encode la
réglementation FFESSM/Code du Sport : certifications, aptitudes, matrices de
prérogatives, contraintes d'âge et prérequis. Zéro dépendance base de données,
zéro I/O — uniquement `serde` (codes stables) et `chrono` (âges/dates).
Consommateurs : module `members` (niveaux structurés), module `palanquees`
(validation de palanquées), importeur Fosse (parsing du blob `diving_level`).

**Hors périmètre** : les arbres de compétences MFT (données configurables par
club, déjà prévus au module `competencies`) ; la lib fournit seulement le
type `Certification` qu'une compétence prépare et la règle « grade minimal du
validateur ».

## 2. Types (les quatre concepts)

```rust
/// Ce qui est DÉLIVRÉ et détenu (avec date d'obtention, stockée par members).
enum Certification {
    // Jeunes
    PlongeurBronze, PlongeurArgent, PlongeurOr,
    // Cursus principal
    Pe12, N1, Pa12, Pe40, Pa20, N2, Pa40, Pe60, N3, N4Gp, N5Dpe,
    // Handisub plongeur
    Pesh6, Pesh12, Pesh20, Pesh40,
    // Gaz
    Nitrox, NitroxConfirme, TrimixElementaire, Trimix,
    // Encadrement fédéral
    Initiateur, Mf1, Mf2, Beppa,
    // Handisub encadrant
    Eh1, Eh2,
    // Diplômes d'État (mapping E de l'annexe III-15b)
    Bpjeps, Bees1, Bees2, Dejeps, DejepsPlongeeProfonde, Desjeps,
    // Vie de club
    Rifap, Tiv, PermisCotier,
}
// NB : PA60 n'existe pas isolément (c'est le N3) — volontairement absent.

/// Ce que le Code du Sport reconnaît (art. A322-77).
enum DiverAptitude { Debutant, Pe12, Pe20, Pe40, Pe60, Pa12, Pa20, Pa40, Pa60 }
enum GasAptitude   { Air, Nitrox40,  NitroxAll, Pth70, Pth120 }
enum SupervisionGrade { Gp, E1, E2, E3, E4 }          // annexe III-15b
enum DpContext { PoolMax6m, ExplorationAirNitrox, Teaching, TrimixShallow, TrimixAll }

/// Statuts transitoires datés (pas des brevets).
enum TrainingStatus { StagiairePedagogiqueMf1 { livret_valide_jusqu_a: Date } }

struct Person<'a> {           // vue minimale fournie par l'appelant
    certifications: &'a [Certification],
    statuses: &'a [TrainingStatus],
    birthdate: Date,
}
```

Chaque `Certification` expose : `code()` (stable, stockage BDD : "N1",
"PA20", "NITROX_C"…), `label_fr()`, `parse(code)`.

## 3. Les matrices (fonctions pures + tables constantes)

Chaque table est une constante documentée avec **sa source (fiche MFT +
édition, ou article/annexe CdS) en commentaire**, pour l'auditabilité.

1. **Aptitudes conférées** — `aptitudes(&Person, on: Date) -> AptitudeSet`
   N1→{PE20} ; PA12→{PA12} ; N2→{PA20, PE40} ; N3→{PA40, PE60, PA60} ;
   PESH-X→PE-X spécial (jamais autonome) ; jeunes→espaces jeunes ; cumul de
   toutes les certifications détenues. Les aptitudes sont **ordonnées par
   profondeur** (`Pe60.covers(Pe20) == true`) mais on ne dérive PAS PA→PE ni
   PE→PA (on suit strictement III-14b).
2. **Aptitudes exerçables** — `exercisable(&Person, on: Date) -> AptitudeSet`
   Applique les âges d'exercice (A322-73/89 + MFT) : PA12/PA20 dès 16 ans,
   PA40 dès 17, PA60 dès 18 ; autonomie mineurs = air/nitrox seulement ;
   restrictions PE40 < 16 ans (sans palier, 2 plongées/j…) exposées en
   `Vec<ExerciseRestriction>` structurées.
3. **Grade d'encadrement** — `supervision_grade(&Person, on) -> Option<Grade>`
   Initiateur→E1 ; Initiateur+N4→E2 ; StagiairePedagogiqueMf1 (livret
   valide)→E2 ; MF1/BEES1/DEJEPS→E3 ; MF2/BEPPA/BEES2/DEJEPS+CC/DESJEPS→E4 ;
   N4 seul→GP. **Gate transitoire : aucun grade exerçable avant 18 ans**
   (MFT N4 mai 2026), constante marquée `// transitional`.
4. **Éligibilité** — `requirements(cert) -> &[Requirement]` et
   `check_eligibility(cert, &Person, facts: &ExperienceFacts, on) ->
   Vec<UnmetRequirement>`.
   `Requirement` = MinAge(u8) | Holds(Certification) | HoldsAny(&[…]) |
   Rifap | DiveCount{count, constraint} | ParentalAuthorization |
   MedicalCertificate | PermisCotier | SignedAptitudes{…}.
   Les faits d'expérience (nombres de plongées attestées) sont fournis par
   l'appelant — la lib ne stocke rien.
5. **Palanquée (exploration & enseignement)** —
   `validate_palanquee(kind: Exploration|Teaching, depth_m, members:
   &[PalanqueeMember], guide: Option<&Person>) -> Result<(), Vec<PalanqueeViolation>>`
   Encode III-16a/16b : effectifs max (4 encadrés +1 GP/P4, 2-3 autonomes,
   baptême = 1), encadrant minimal par espace (0-60 encadré ⇒ E4), aptitude
   la plus restrictive, autonomie sans encadrant, âges d'exercice.
6. **Directeur de plongée** — `can_be_dp(&Person, ctx: DpContext, on) -> bool`
   (III-15a + A322-98 : E1 piscine ≥ 18 ans ; explo air/nitrox : N5/DPE ;
   enseignement : E3 ; trimix profond : E4 ; mélanges : +PN-C/PTH-120).
7. **Gaz** — `max_o2_fraction(&Person) -> Option<f32>` (PN 40 %, PN-C 100 %
   déco) ; utilitaire `nitrox_mod(o2_fraction, ppo2_max) -> depth` avec les
   bornes légales PpO₂ 0,16–1,6 bar (A322-92) ; profondeur air/nitrox ≤ 60 m.
8. **Progression** — `eligible_next(&Person, on) -> Vec<Certification>`
   (remplace proprement le `getPreparingOptions` frontend de Fosse) et un
   ordre partiel `display_rank()` pour l'affichage « plus haut niveau ».
   Affiné (retour produit 2026-08-18) :
   - `implied_certifications` : fermeture des prérequis `Holds`/
     `HoldsForDelivery` — détenir un N2 **prouve** le N1 ; les possessions
     impliquées satisfont les prérequis et ne sont jamais proposées.
   - `is_redundant` : exclut toute certification qui n'ajouterait **aucune
     prérogative** (aptitudes plongeur/gaz, encadrement) — un N2 rend
     PE12/PA12/PE40/PA20 sans objet, un MF1 rend l'Initiateur sans objet —
     SAUF si elle garde une **valeur de chaîne** (prérequis strict d'une
     certification encore utile : le Plongeur d'Or a toujours besoin du N1
     pour viser le N2). Les qualifications hors matrices (RIFAP, TIV,
     bronze/argent, EH1/EH2…) ne sont jamais redondantes.
   - `Person.handisub` : les PESH ne sont proposés qu'aux personnes en
     situation de handicap (flag, ou PESH déjà détenu). Les EH restent
     ouverts à tous les encadrants.
   - `Requirement::HoldsForDelivery` : condition d'examen, ne bloque pas
     l'entrée en cursus (Initiateur : RIFAP exigé à l'examen — MFT déc.
     2025 : stage initial dès 15 ans + PA20).
   - `preparable_certifications` : liste pour le sélecteur « en
     préparation » — mêmes exclusions, mais tolère les prérequis d'entrée
     non encore acquis (le club décide), en excluant l'impossible
     (`MaxAge` dépassé).
   - **Absorption des modules** (produit 2026-08-18) : dans `eligible_next`,
     quand un brevet complet atteignable **domine strictement** un module
     (couvre tout ce qu'il apporterait, et plus), seul le brevet est
     proposé — un N2 voit « N3 », pas « PA40 + PE60 » ; un N1 adulte voit
     « N2 » ; un N1 de 14 ans voit « PE40 » (N2 à 15 ans). Exceptions :
     hors-matrices (RIFAP, TIV…) et **PESH** (niveaux d'entrée choisis
     selon les capacités de la personne, pas une échelle de modules).
   - **Conditions de délivrance** (`HoldsForDelivery`, confirmé produit
     2026-08-18 : « il le faut pour être certifié, pas pour commencer la
     formation ») : RIFAP pour Initiateur/N3/N4/N5/MF1/MF2/EH2 ; permis
     côtier pour MF1/MF2 ; PN-C pour MF2. Remontées « à vérifier », ne
     bloquent jamais la proposition — un N4 voit donc MF1, N5 et
     Initiateur.
   - L'implication traverse `HoldsAny` par **intersection des fermetures**
     des alternatives : « N2 ou PA40 » prouve le N2 (PA40 exige N2) ; « N4
     ou BPJEPS » ne prouve rien. Un N4 nu en base implique N3→N2→N1+RIFAP.
   - La vue de couverture (redondance + absorption) inclut les **contextes
     DP** : le N5 n'est pas « un moins bon MF1 » (sa valeur est le DP
     exploration) et n'est ni redondant ni absorbé pour un N4.

## 4. Versionnage réglementaire

La réglementation change (démonstration : arrêtés d'oct./nov. 2025). La lib
porte une constante `REGULATION_SNAPSHOT: &str = "MFT 2025-2026 / CdS
2026-01-17"` retournée par l'API ; toute constante est sourcée en commentaire
et tout changement réglementaire = PR dédiée mettant à jour tables + tests +
snapshot. Les points transitoires (N4 18 ans) et réserves (III-15b verbatim)
sont marqués `// RESERVE:` avec le lien vers l'analyse.

## 5. Plan de tests (« ultra testé »)

- **Table-driven exhaustif** : un cas de test par cellule de chaque matrice —
  toutes les certifications × aptitudes conférées ; tous les âges charnières
  (obtention ET exercice : 11/12, 13/14, 14/15, 15/16, 16/17, 17/18, 19/20
  ans) testés à J-1/J/J+1 de l'anniversaire ; tous les prérequis (présents,
  absents, RIFAP manquant, plongées insuffisantes).
- **Palanquées** : chaque ligne de III-16a/16b (encadrant exact, encadrant
  insuffisant, effectif max, +1 GP/P4, mix d'aptitudes → plus restrictive,
  autonomes mineurs, PE-60 sans E4, baptême à 2 élèves → violation).
- **DP** : chaque contexte × chaque grade (y compris E1 de 17 ans en
  piscine → refus).
- **Propriétés** (proptest) : monotonie (ajouter une certification n'enlève
  jamais d'aptitude ; vieillir n'enlève jamais d'aptitude exerçable) ;
  cohérence covers() (PE60 ⊇ PE40 ⊇ PE20 ⊇ PE12) ; round-trip code/parse.
- **Cas réels** : une dizaine de personas (l'Or de 12 ans, le N2 de 15 ans,
  l'Initiateur-GP, le MF1, le PESH-20 nitrox, le N3 mineur…) vérifiés
  bout-en-bout contre les fiches MFT.
- Objectif : la crate est LE filet de sécurité réglementaire — viser une
  couverture proche de 100 % des tables.

## 6. Questions ouvertes (avant implémentation)

1. **Périmètre v1** : inclure jeunes (Bronze/Argent/Or), Trimix et diplômes
   d'État dès la v1 ? (Coût faible — recommandé oui pour ne pas re-migrer.)
2. Statut **stagiaire pédagogique MF1** avec date de livret : utile pour
   l'USI ? (Recommandé oui, c'est le seul E2 temporaire.)
3. **RIFAP/TIV/permis bateau** trackés comme certifications (avec dates de
   recyclage TIV 5 ans) — confirmer.
4. Les restrictions Handisub cliniques (profil PESH-40, 40 min…) : simples
   constantes documentées ou validation active dans `validate_palanquee` ?
   (Recommandé : constantes + violation « profil PESH-40 » en warning.)
