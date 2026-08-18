# Rôles d'autorisation vs grades d'encadrement — analyse de la proposition « supprimer le type encadrant »

Status : **proposition** (2026-08-18). Répond à la demande produit : « On a 3
types de user (admin/encadrant/membre). Je pense qu'on peut se passer
d'encadrant : ce type peut être automatiquement défini lorsqu'un user a un
niveau E. Ça simplifie l'UX énormément. »

## 1. Reformulation du besoin

Le mot « encadrant » recouvre aujourd'hui **deux concepts distincts** :

| Concept | Nature | Où il vit | Ce qu'il donne |
|---|---|---|---|
| **Rôle « Encadrant »** | autorisation applicative, **par club** | `club_roles` seedé à la création du club (`tenancy/services/seed.rs`) | des permissions UI/API — par défaut une seule : `club.members.view` |
| **Grade E1–E4** | capacité **réglementaire**, propre à la personne | dérivé à la lecture par la crate `ffessm` (`supervision()`) depuis les certifications/statuts du profil user-global | ce que la personne a le droit de faire sous l'eau : enseigner, guider, être DP |

La proposition produit vise le second concept (le grade) mais le mot « type de
user » désigne le premier (le rôle). La confusion vient de Fosse, qui fusionnait
les deux dans ses « groupes », et du fait que le rôle seedé porte le même nom
que le grade. Or dans Dive Manager le découplage existe déjà en partie : les
badges de la liste membres, la section encadrant du profil (numéro + tampon,
réservé `teaching_certified ≥ E2`) et les drapeaux DP sont **déjà dérivés du
grade**, sans regarder le rôle. Le rôle « Encadrant » ne sert aujourd'hui qu'à
voir la liste des membres.

Le vrai besoin reformulé : **ne pas obliger l'admin à maintenir à la main une
information (qui est encadrant) que l'app sait déjà calculer** — sans pour
autant transformer une donnée fédérale en source de permissions.

## 2. Analyse de sécurité du couplage automatique (rôle ⇐ grade)

Si les permissions découlaient automatiquement du grade E :

1. **Escalade indirecte : la saisie de données devient une attribution de
   droits.** Les certifications sont saisies par le staff du club sous
   `club.members.manage` (décision 2026-08-18 : pas d'auto-déclaration).
   Quiconque peut enregistrer un MF1 « carte vue » (provenance `recorded`)
   fabriquerait un encadrant avec ses permissions — alors que l'attribution de
   rôles est aujourd'hui gardée séparément par `club.roles.manage`. Deux
   frontières de sécurité distinctes (données vs autorisations) fusionneraient
   silencieusement.

2. **Fuite inter-clubs.** Le profil plongée est **user-global** (décision
   2026-08-17) : un E3 qui adhère comme simple membre à un second club y
   obtiendrait automatiquement des permissions que ce club ne lui a jamais
   accordées. C'est contraire à ADR-0003 (« permissions = grants explicites »)
   et à l'isolation des autorités par tenant : le club B subirait une décision
   du club A (ou de la fédération).

3. **Révocation impossible proprement.** Pour retirer les droits d'un
   encadrant problématique, il faudrait supprimer son brevet — une donnée
   fédérale **vraie** qu'on n'a aucune raison de falsifier. Seule alternative :
   suspendre l'adhésion entière (trop grossier). Un rôle, lui, se change en un
   clic, audité, effet immédiat (sessions serveur, ADR-0004).

4. **Provenance : quel niveau de confiance suffit ?** `recorded` = un membre du
   staff dit avoir vu la carte ; `imported` = blob Fosse ; `validated`
   n'existera qu'avec le module `competencies`. Dériver des permissions de
   `recorded`/`imported` fait reposer l'autorisation sur une saisie non
   vérifiée ; exiger `validated` rendrait la dérivation inopérante pendant des
   mois (tout l'existant est `imported`/`recorded`). Note : le tampon accepte
   aujourd'hui un grade issu de `recorded` — acceptable car le tampon n'est pas
   une permission applicative ; ce raisonnement ne s'étend pas aux droits.

5. **Stagiaire pédagogique MF1.** Statut daté, E2 **enseignement seulement**,
   jamais DP (`teaching` ≠ `teaching_certified` dans la lib). Compte-t-il comme
   « niveau E » ? Si oui : des permissions qui expirent toutes seules à la date
   du livret (comportement surprenant, jamais vu ailleurs dans l'app) et un
   « encadrant » applicatif dont les droits excéderaient sa capacité
   réglementaire. Si non : deux E2 affichés identiques auraient des droits
   différents. Les deux réponses sont mauvaises — signe que la question est mal
   posée.

Nuance ADR-0003 : dériver des droits de données ne viole pas « l'absence de
donnée ne donne jamais accès », mais viole « les permissions sont des grants
explicites ». La présence d'une donnée métier deviendrait un grant implicite.

## 3. Options

| | A. Statu quo | B. Proposition brute | C. Découplage strict | D. Automatisation assistée |
|---|---|---|---|---|
| Rôle Encadrant seedé | oui | non | **non** | oui |
| Source des permissions | rôle | grade E | rôle | rôle |
| Source de la capacité métier (DP, palanquées, validation) | grade (déjà) | grade | grade | grade |
| UX admin | 3 rôles à gérer, double saisie | zéro gestion | 2 rôles, zéro double saisie | 3 rôles + suggestions |
| Sécurité | saine mais concepts confondus | §2 : escalade, fuite inter-clubs, révocation | saine, frontières nettes | saine (grant explicite, un clic, audité) |
| Coût code | 0 | moteur de dérivation de permissions + gestion stagiaire/provenance/multi-clubs | ~suppression de 10 lignes de seed | bandeaux + action auditée + détection d'incohérence |

### A. Statu quo
Deux concepts homonymes coexistent ; l'admin doit à la fois saisir le MF1 **et**
penser à passer le rôle Encadrant — la double saisie que le product owner veut
supprimer. Le rôle ne donne pourtant que `club.members.view`. Complexité UX
réelle pour un bénéfice quasi nul.

### B. Proposition brute (permissions dérivées du grade)
Mérites honnêtes : zéro maintenance, jamais d'oubli, cohérence
affichage/droits garantie, un concept de moins à expliquer. Mais tous les
problèmes du §2 sont bloquants, et il faudrait quand même définir *quelles*
permissions un grade E confère — décision qui varie par club (certains clubs
voudront que les encadrants voient les coordonnées des membres, d'autres non).
La simplification apparente déplace la complexité vers des cas tordus
(multi-clubs, stagiaire, révocation) au lieu de la supprimer.

### C. Découplage strict (recommandée)
Les **permissions** restent aux rôles ; le seed passe à deux rôles
(Administrateur, Membre) — l'UX voulue par le product owner : plus de « type
encadrant » à gérer. La **capacité métier** est toujours dérivée du grade
ffessm dans les modules métier — le rôle n'y joue aucun rôle (c'est déjà le
design : voir §4). La question résiduelle « un encadrant voit-il la liste des
membres ? » devient un choix de permission par club (accorder
`club.members.view` au rôle Membre, ou créer plus tard un rôle personnalisé).
On obtient ~tous les mérites UX de B sans ses failles : le grade est déjà
affiché partout automatiquement ; il n'y a simplement plus rien à synchroniser.

### D. Automatisation assistée
Le rôle reste la source d'autorisation, mais l'app **propose** (bandeau sur la
fiche membre, action un-clic auditée, `require_real + club.roles.manage`) de
passer un membre en Encadrant quand un grade E certifié est enregistré, et
signale l'incohérence inverse (rôle Encadrant sans grade). Révocation normale.
Sûre et ergonomique, mais elle conserve trois rôles et du code de suggestion
pour synchroniser deux concepts… dont C démontre qu'un seul suffit. À ne
retenir que si le product owner tient à un rôle Encadrant porteur de vraies
permissions distinctes.

Hybride envisageable : **C maintenant, D plus tard** si des permissions
spécifiques encadrant apparaissent (le mécanisme de rôles par club les couvre
sans re-design).

## 4. Impact sur l'existant (option C)

| Zone | Impact |
|---|---|
| Seed (`tenancy/services/seed.rs`) | supprimer `ROLE_INSTRUCTOR` et son grant `club.members.view` ; nouveaux clubs = 2 rôles |
| Clubs existants | le rôle Encadrant existant reste (inoffensif : `members.view` seulement) ; l'admin réaffecte ses titulaires vers Membre quand il veut — pas de migration destructive |
| Page Rôles | inchangée (elle liste ce qui existe) ; 2 lignes au lieu de 3 pour les nouveaux clubs |
| Badges liste membres | **déjà dérivés** du grade (`teaching`, `gp`, RIFAP, CACI) — aucun changement |
| Invitations / ajout direct | choix de rôle réduit à Admin/Membre — plus simple |
| Section encadrant du profil (numéro + tampon) | **déjà gatée** par `teaching_certified ≥ E2` — aucun changement, et c'est la preuve que le pattern fonctionne |
| `activities` (DP) | affectation DP = capacité `can_be_dp(person, ctx)` ; le droit d'affecter = permission du module (ex. `sessions.manage`). Aucun besoin du rôle |
| `palanquees` | `validate_palanquee` prend des `Person` et vérifie les grades — aucun besoin du rôle |
| `competencies` | gate « grade minimal du validateur » = `TeachingGrade(...)` de la lib — aucun besoin du rôle ; le droit d'ouvrir l'écran de validation = permission du module |

Démonstration : **aucun module futur ne consomme le rôle Encadrant** — la
capacité dérivée suffit partout où « être encadrant » compte fonctionnellement.

## 5. Recommandation

**Option C — découplage strict.** Autorisation = rôles (grants explicites,
révocables, par club, ADR-0003 respecté à la lettre : rien n'y déroge — on
*renforce* la règle en refusant un grant implicite). Capacité réglementaire =
grade ffessm, dérivé, jamais stocké, jamais source de permissions.

Plan de migration minimal :
1. Retirer le rôle Encadrant du seed (nouveaux clubs).
2. Ne rien toucher aux clubs existants ; documenter que le rôle résiduel peut
   être vidé par l'admin (réaffectation vers Membre via la page Membres).
3. Importeur Fosse : mapper les anciens « encadrants » vers le rôle Membre —
   leurs grades arrivent par les certifications importées (`E3`→MF1, etc.).
4. Mettre à jour foundation-module.md (seed) et MODULES.md après décision.

Décisions restant au product owner : les quatre questions ci-dessous, plus le
choix par défaut de `club.members.view` pour le rôle Membre.

## 6. Questions ouvertes

1. Le rôle **Membre** doit-il recevoir `club.members.view` par défaut (tout le
   monde voit la liste), ou la liste reste-t-elle réservée aux admins tant
   qu'un club n'en décide pas autrement ? (fail-closed penche pour la seconde)
2. Clubs existants : réaffecter automatiquement les titulaires du rôle
   Encadrant vers Membre lors de la mise à jour, ou laisser l'admin le faire ?
3. Import Fosse : confirmer le mapping « groupe encadrant » → rôle Membre +
   certifications importées (aucun rôle Encadrant recréé) ?
4. Faut-il prévoir dès maintenant la **création de rôles personnalisés** par
   club (le cas « encadrant avec droits supplémentaires » de la note du design
   fondation), ou attendre une demande réelle ?
5. Confirmer que le stagiaire MF1 reste traité comme aujourd'hui : badge E2 et
   encadrement pédagogique oui, tampon/DP non, et aucun droit applicatif ?

## 7. Décisions produit (2026-08-18)

Option **C (découplage strict) acceptée**. Réponses aux questions ouvertes :

1. **Non** — le rôle Membre ne reçoit aucune permission par défaut
   (fail-closed ; un club peut toujours accorder `club.members.view` à
   Membre via la page Rôles).
2. **Oui** — les titulaires actuels du rôle Encadrant sont réaffectés
   automatiquement vers Membre (pré-prod : correction one-off des données
   de dev, aucun environnement déployé).
3. **Oui** — import Fosse : tout le monde arrive en Membre (sauf admins),
   les grades se dérivent des certifications importées.
4. **Pas d'autres rôles** — v1 = Administrateur + Membre uniquement ; pas
   de création de rôles personnalisés (l'édition des permissions du rôle
   Membre reste possible par club).
5. **Oui** — stagiaire MF1 : badge et capacité d'enseignement dérivés,
   jamais DP, aucun droit applicatif.

Statut : **accepté** — implémentation planifiée (après les chantiers
« arbre de certifications » et « section encadrant conditionnée »).
