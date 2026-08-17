# FFESSM — niveaux, aptitudes, encadrement : synthèse métier sourcée

> Établi le 2026-08-17 à partir des **fiches officielles du MFT** (PDF
> téléchargés depuis mft.ffessm.fr, édition notée pour chaque fiche) et du
> **Code du Sport en vigueur** sur Légifrance (art. A322-71 à A322-101 +
> annexes, telles que réécrites par les arrêtés des 17/10/2025 et 24/11/2025,
> en vigueur depuis le 17/01/2026). Les PDF sources sont archivés dans le
> scratchpad de session. Ce document nourrit la lib métier Rust `ffessm`.
>
> ⚠️ **Fraîcheur** : la réglementation a changé fin 2025 (A322-88 abrogé,
> annexes III-15a/b réécrites, âges MFT abaissés). La plupart des sites de
> clubs — et les pages grand public ffessm.fr elles-mêmes — sont périmés.
> Le MFT en vigueur et Légifrance font foi.

## 1. Les quatre concepts (fondement du modèle)

1. **Brevet / qualification (certification)** — ce qui est *délivré* et
   détenu à vie : N1…N5, qualifications PE/PA autonomes, PESH, brevets
   jeunes, Nitrox/Trimix, Initiateur/MF1/MF2, RIFAP, TIV…
2. **Aptitude (Code du Sport, art. A322-77)** — ce que le plongeur est
   *apte à faire* : PE-12/20/40/60 (encadré), PA-12/20/40/60 (autonome),
   PN/PN-C (nitrox), PTH (trimix). Les brevets **confèrent** des aptitudes
   (annexe III-14b) ; à défaut de brevet, le DP peut évaluer l'aptitude en
   plongée.
3. **Prérogatives opérationnelles** — dérivées des aptitudes par les
   tableaux du Code du Sport : espaces d'évolution, autonomie ou non,
   encadrant minimal, effectifs de palanquée, conditions d'âge d'exercice.
4. **Compétences (MFT)** — le contenu pédagogique de chaque brevet en cours
   de préparation. Données de formation (arbres par club dans Dive Manager),
   PAS des règles de la lib — sauf le lien « compétence → brevet préparé »
   et le niveau minimal du validateur.

**Détenir ≠ pouvoir exercer** : un mineur peut détenir PA20 à 15 ans mais ne
l'exercer qu'à 16 ; un N4 de 17 ans n'encadre qu'à 18. La lib doit séparer
`aptitudes détenues` et `aptitudes exerçables (à la date D, à l'âge A)`.

## 2. Aptitudes et espaces d'évolution (Code du Sport)

- **Espaces** (A322-76) : 0-6, 0-12, 0-20, 0-40, 0-60 m (au-delà : trimix,
  0-70/0-80/+80). **L'air et le nitrox sont limités à 60 m.**
- **Aptitudes** (annexe III-14a) : PE-X = évoluer en palanquée **encadrée**
  dans l'espace 0-X ; PA-X = en **autonomie**. PE-60/PA-60 réservés aux
  titulaires d'un brevet (FFESSM/FSGT/UCPA/ANMP/SNMP/CMAS).
- **Correspondance brevets → aptitudes** (annexe III-14b) : P1 → PE-20 ;
  « P1 incluant l'autonomie » → PE-20 + PA-12 ; P2 → PE-40 + PA-20 ;
  P3 → PE-60 + PA-60.
- **Âges d'exercice de l'autonomie** (A322-73, réécrit 17/10/2025 ;
  l'ex-A322-88 est **abrogé**) : PA-12 et PA-20 dès **16 ans**, PA-40 dès
  **17 ans**, sur décision du DP ; PA-60 : **majeurs** (A322-89). Autonomie
  des mineurs **limitée à l'air et au nitrox**.
- **Palanquée** (A322-73) : les caractéristiques de la plongée respectent
  les **aptitudes les plus restrictives** de la palanquée.

## 3. Cursus plongeur (fiches MFT en vigueur)

| Brevet / qualif | MFT (édition) | Âge min (obtention) | Prérequis | Aptitudes conférées | Exercice / restrictions |
|---|---|---|---|---|---|
| **PE12** | mai 2024 | **10 ans** | aucun (ni licence ni CACI !) ; autor. parentale | PE-12 | découverte ; 1ʳᵉ immersion ≤ 3 m pour 10-14 ans |
| **N1** | déc. 2025 | **12 ans** | licence + CACI ; autor. parentale mineurs | **PE-20** (pas de PA-12 inclus) | si certifié en milieu artificiel : 4 plongées mer sous 12 mois |
| **PA12** | mai 2026 | **15 ans** | N1 + 4 plongées mer ; licence + CACI | PA-12 | exercice à partir de **16 ans** ; mineur : autor. éclairée + info palanquée |
| **PE40** (module N2) | mai 2026 | **14 ans** | N1 + 4 plongées mer | PE-40 | < 16 ans : sans palier obligatoire, ≤ 2 plongées/j espacées ≥ 3 h, 2ᵉ ≤ 20 m si 1ʳᵉ > 30 m |
| **PA20** (module N2) | mai 2026 | **15 ans** | N1 + 4 plongées mer | PA-20 | exercice à partir de **16 ans** |
| **N2** = PA20+PE40 | mai 2026 | **15 ans** | N1 + 4 plongées mer ; licence + CACI | **PA-20 + PE-40** | modules capitalisables dans un ordre libre ; pas de RIFAP exigé |
| **PA40** (module N3) | déc. 2025 | **16 ans** | **N2** | PA-40 | exercice à partir de **17 ans** |
| **PE60** (module N3) | déc. 2025 | **17 ans** | **PE40 + 20 plongées mer dont 5 entre 35-40 m** | PE-60 | encadrant **E4** entre 40 et 60 m |
| **N3** | déc. 2025 | **17 ans** | **N2 ou PA40** + **RIFAP** ; CACI | **PA-40 + PE-60 + PA-60** | autonomie > 40 m à partir de **18 ans** ; 0-40 m sans DP possible (A322-99) ; **PA60 n'existe pas en qualification isolée : c'est le N3** |
| **N4-GP** | mai 2026 | **17 ans (examen)** | **N3** (la passerelle PA40+RIFAP de 2023 est supprimée) + RIFAP + 4 aptitudes signées E3 (validité 9 mois) | GP (encadrement exploration) | ⚠️ **exercice de l'encadrement à partir de 18 ans** (transitoire, « dans l'attente de la Direction des Sports ») |
| **N5-DPE** | mai 2026 | **18 ans** | GP-N4 + RIFAP + **15 plongées en situation de GP** ; délivré en club par E3 min | DP exploration (air/nitrox) | sous responsabilité du président de club ; pas d'enseignement |

**Divergences signalées** (les pages grand public ffessm.fr contredisent le
MFT — le MFT fait foi) : N1 « 14 ans sauf dérogation » (périmé, c'est 12) ;
N2 « PE40 16 ans / PA20 18 ans » (périmé, c'est 14/15 avec exercice à 16).

### Jeunes plongeurs (fiche MFT août 2023)

Scaphandre dès **8 ans**. Bronze/Argent dès 8 ans, Or dès 10 ans (pas de
chaînage obligatoire). Espaces : baptême 0-2 m (8-10 ans) / 0-3 m (10-14) ;
Bronze & Argent 0-6 m ; **Or 0-12 m (10-12 ans) puis 0-20 m (12-14 ans)**.
Contraintes : plongées **dans la courbe de sécurité**, **1 plongée/jour
jusqu'à 12 ans**, autorisation parentale, licence, CACI (sauf baptême),
matériel adapté. L'Or ≈ compétences N1 ; N1 accessible dès 12 ans.

### Handisub — PESH (fiche MFT mai 2025)

PESH-6 (dès **8 ans**), PESH-12 (**10**), PESH-20 (**12**), PESH-40 (**18**,
prérequis PESH-20 + 10 plongées zone 20 m signées EH1). **Jamais
d'autonomie.** Encadrement par E1/E2 + qualification Handisub (EH1 handicap
modéré, EH2 majeur) ou GH. Contraintes : zone sans palier obligatoire,
second moyen de donner de l'air sans partage d'embout, PESH-40 : profil
strict (profondeur max en début de plongée, ≤ 40 min, fin vers 6 m). La
ventilation doit toujours rester volontaire. PESH-20 ouvre le nitrox
élémentaire. Base légale : assistance adaptée PE-12→PE-40 (A322-77).

### Qualifications gaz (fiches MFT mai 2024 / oct. 2017)

| Qualif | Âge | Prérequis | Prérogatives |
|---|---|---|---|
| **Plongeur Nitrox (PN)** | 12 ans | N1 | mélanges **≤ 40 % O₂**, dans les limites de son niveau air |
| **Plongeur Nitrox Confirmé (PN-C)** | 15 ans | N2 + PN | tous nitrox + **O₂ pur en déco** |
| **Trimix élémentaire (PTH-70)** | 18 ans | N3 + PN-C + 15 plongées > 40 m | trimix jusqu'à **70 m** |
| **Trimix (PTH-120)** | 18 ans | + Trimix élém. + 10 plongées trimix qualifiantes | jusqu'à **120 m** |

Bornes physiologiques légales : **PpO₂ 0,16–1,6 bar** (A322-92) ; nitrox et
air limités à 60 m ; encadrement trimix ≤ 80 m, autonomie ≤ 120 m (A322-76).

## 4. Encadrement (fiches MFT + annexe III-15b du 17/01/2026)

| Grade | Brevets fédéraux | Diplômes d'État | Enseigne (III-16a) |
|---|---|---|---|
| **GP** | GP / P4 | BPJEPS (et stagiaire BPJEPS scaphandre) | — (exploration seulement) |
| **E1** | Initiateur | BPJEPS | 0-6 m (débutants, baptêmes) |
| **E2** | **Initiateur + GP** ; **stagiaire pédagogique MF1** (livret 3 ans) ; CMAS M1★ | BPJEPS | 0-12 et 0-20 m |
| **E3** | **MF1** ; CMAS M2★ | BEES1, **DEJEPS plongée**, stagiaire DESJEPS | 0-40 m |
| **E4** | **MF2**, **BEPPA** | BEES2, DEJEPS + CC « plongée profonde et tutorat », DESJEPS | 0-60 m |

⚠️ Réserve : libellés exacts III-15a/b lus par extraction automatique
(structure recoupée avec l'arrêté du 24/11/2025) ; la position précise du
DEJEPS ancien libellé en E4 mérite une relecture humaine avant gravure.

### Fiches encadrants (éditions MFT)

- **Initiateur (E1)** — déc. 2025 : stage initial dès **15 ans + PA20** ;
  examen dès **16 ans** + RIFAP + 12 plongées post-PA20 (dispense si
  N3/N4/N5). Prérogatives : DP piscine/fosse ≤ 6 m **à partir de 18 ans** ;
  enseigne 0-6 m (milieu naturel : sous DP E3) ; valide le N1 en milieu
  artificiel. *(L'ancien « 18 ans + N2 » est périmé.)*
- **E2 = Initiateur+GP** : enseigne 0-20 m sous direction d'un E3 ; valide
  N1 et N2 (N2 co-signé E3). Module 6-20 m recommandé. Le **stagiaire
  pédagogique MF1** enseigne 0-20 m sous DP E3 mais ne délivre rien et
  n'est jamais DP.
- **MF1 (E3)** — juil. 2026 : **18 ans**, GP-N4 + RIFAP + **permis bateau
  côtier** ; stage initial 35 h → stage péda → stage final → examen (3 ans
  max ; filière « capitalisation » possible). Prérogatives : enseigne
  0-40 m ; **DP toutes situations air/nitrox (exploration ET
  enseignement)** ; valide N1-N3, PA/PE, aptitudes N4, qualifs DP-N5 ;
  jurys N4/Initiateur ; tuteur TSI.
- **MF2 (E4)** — mai 2026 : **20 ans** (à l'examen), MF1 depuis ≥ 12 mois,
  **PN-C exigé** (nouveauté), permis bateau, aptitudes A1-A5 signées E4.
  Prérogatives : enseigne 0-60 m ; formateur de cadres (tutorat MF1/MF2,
  stages initiaux, jurys) ; DP y compris trimix (avec qualif).
- **Handisub** : EH1 (E1 min + 5 plongées d'enseignement, stage 24 h) ;
  EH2 (18 ans, E2 min + EH1 + 5 encadrements, stage 5 j) ; GH (GP qualifié) ;
  MFEH1/2 (formateurs).
- **Moniteur Nitrox Confirmé** : 18 ans, E3 min + PN-C. **Moniteur
  Trimix** : E4 + qualif Trimix.
- **Vie de club** : **RIFAP** (secourisme fédéral, exigé pour N3, N4, N5,
  Initiateur, MF1, MF2, EH1/2 ; réactualisation conseillée ≤ 3 ans, pas de
  péremption formelle) ; **TIV** (18 ans, inspection bouteilles, **recyclage
  obligatoire 5 ans**) ; **permis bateau côtier** (titre d'État, requis MF1/MF2).

## 5. Directeur de plongée (annexe III-15a du 17/01/2026 + A322-72/98/99)

| Contexte | DP minimal |
|---|---|
| Piscine / fosse ≤ 6 m | **E1** (Initiateur : 18 ans min) ; fiche de sécurité facultative ; peut autoriser PE-12 en autonomie et GP/P4 aux baptêmes |
| Exploration air/nitrox (milieu naturel) | **DPE / P5 (N5)** |
| Enseignement air/nitrox ; trimix ens. ≤ 40 m & explo ≤ 70 m | **E3** (MF1/BEES1/DEJEPS/DESJEPS) |
| Trimix/héliox toutes profondeurs | **E4** (MF2/BEPPA/BEES2/…) |
| Mélanges (tous contextes) | + justifie **PN-C ou PTH-120** |

Responsabilités (A322-72) : présent au lieu de mise à l'eau, organise et
sécurise, fixe les caractéristiques des plongées, **établit la fiche de
sécurité (conservée 1 an)**. **Exception sans DP** (A322-99) : palanquée de
PA-60 en 0-40 m, sur décision de l'exploitant qui entérine l'organisation.

## 6. Palanquées — effectifs et encadrants (annexes III-16a/b, vérifiées)

**Exploration (III-16b)** — effectif hors encadrant, (*) = +1 possible si
GP/P4 :

| Espace | Encadrés | Max | Encadrant min | Autonomes | Max |
|---|---|---|---|---|---|
| 0-6 m | Débutants | 4 (*) | E1 ou GP/P4 | — | — |
| 0-12 m | PE-12 | 4 (*) | E2 ou GP/P4 | PA-12 | 3 |
| 0-20 m | PE-20 | 4 (*) | E2 ou GP/P4 | PA-20 | 3 |
| 0-40 m | PE-40 | 4 (*) | E3 ou GP/P4 | PA-40 | 3 |
| 0-60 m | PE-60 | 4 | **E4** | PA-60 | 3 |

**Enseignement (III-16a)** : baptême = 1 élève (E1) ; débutants 0-6 m = 4
(E1) ; formation PE/PA-12 et -20 = 4 (E2) ; -40 = 4 (E3) ; -60 = 4 (**E4**) ;
même règle (*).

## 7. Certificat médical (CACI — synthèse officielle FFESSM)

- Plongée scaphandre (air, nitrox, trimix élémentaire) : **CACI < 1 an
  obligatoire, adultes ET mineurs**, par tout médecin (le questionnaire de
  santé mineurs ne s'applique PAS au scaphandre).
- **Médecin fédéral / DU-DIU / médecin du sport requis** pour : trimix
  hypoxique, reprise après accident de plongée, premier CACI d'un PESH à
  troubles physiques/sensoriels.
- **Sans licence ni CACI** : baptême, **PE12**, Pass Découverte.
- Modification de l'état de santé ou accident ⇒ validité suspendue.

## 8. Réserves et points de veille

1. Libellés verbatim des annexes III-15a/b (extraction automatisée) et
   position du DEJEPS ancien intitulé en E4 — relecture humaine conseillée.
2. **N4 à 17 ans, exercice à 18** : mesure transitoire MFT (« dans l'attente
   de la décision de la Direction des Sports ») — à re-vérifier.
3. Aucune fiche MFT ne prévoit d'expiration des brevets (seuls CACI, TIV
   5 ans, aptitudes d'examen 9 mois, livrets pédagogiques 3 ans expirent).
4. Fiche Trimix ancienne (oct. 2017) mais bien en vigueur au catalogue MFT.
5. La réglementation ayant changé fin 2025, la lib doit porter la **date de
   snapshot réglementaire** et sourcer chaque constante.

## 9. Sources principales

MFT (mft.ffessm.fr, PDF officiels ; éditions citées dans le texte) : fiches
N1, N2, N3, GP-N4, DPE-N5, PA12, PE12, Jeunes plongeurs, Handisub PESH,
Handisub encadrants, Nitrox, Trimix, Initiateur, MF1, MF2, RIFAP, TIV,
Généralités (janv. 2025), « Extrait du Code du Sport » (juin 2026).
Code du Sport (Légifrance, en vigueur) : A322-72, -73, -76, -77, -83 à -87,
-89, -92, -98 à -101 ; annexes III-14a/b, III-15a/b (17/01/2026),
III-16a/b, III-17a, III-18a ; arrêtés du 17/10/2025 et 24/11/2025.
Médical : tableau CACI officiel FFESSM (ffessm.fr / medical.ffessm.fr).
