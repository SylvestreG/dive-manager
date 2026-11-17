// French UI copy — the single dictionary (translation-layer embryo).
// Components never hardcode user-facing strings.
export const t = {
  appName: "Dive Manager",

  nav: {
    home: "Accueil",
    club: "Mon club",
    competencies: "Compétences",
    platform: "Plateforme",
    profile: "Profil",
  },

  common: {
    loading: "Chargement…",
    retry: "Réessayer",
    cancel: "Annuler",
    save: "Enregistrer",
    send: "Envoyer",
    create: "Créer",
    close: "Fermer",
    error: "Une erreur est survenue.",
    emptyList: "Rien à afficher pour le moment.",
    email: "Adresse email",
    firstName: "Prénom",
    lastName: "Nom",
    active: "Actif",
    suspended: "Suspendu",
    disabled: "Désactivé",
    total: (n: number) => `${n} au total`,
    previous: "Précédent",
    next: "Suivant",
    pageOf: (page: number, pages: number) => `Page ${page} / ${pages}`,
    role: "Rôle",
    selectRole: "Choisir un rôle…",
  },

  login: {
    title: "Connexion",
    intro: "Accédez à votre espace club.",
    sendLink: "Recevoir un lien de connexion",
    magicLinkSent:
      "Si cette adresse est connue, un lien de connexion vient d'être envoyé par email. Il est valable 12 heures.",
    changeEmail: "Changer d'adresse",
    or: "ou",
    continueWith: (provider: string) => `Continuer avec ${provider}`,
    emailOption: "Recevoir un lien par email",
    errors: {
      sso: "La connexion n'a pas abouti. Réessayez ou utilisez le lien par email.",
      sso_cancelled: "Connexion annulée.",
      rate_limited: "Trop de tentatives, réessayez dans quelques minutes.",
    } as Record<string, string>,
    privacy: "Confidentialité",
    tooManyAttempts: "Trop de tentatives, réessayez dans quelques minutes.",
  },

  privacy: {
    title: "Politique de confidentialité",
    updated: "Dernière mise à jour : 17 novembre 2026",
    sections: [
      {
        title: "Qui sommes-nous ?",
        paragraphs: [
          "Dive Manager est un service de gestion de clubs de plongée, exploité par [à compléter : nom de la personne ou de la structure], [à compléter : adresse]. Contact : [à compléter : email de contact].",
          "Chaque club utilisateur reste responsable des données de ses membres qu'il saisit ou importe ; l'exploitant agit pour son compte.",
        ],
      },
      {
        title: "Données traitées",
        paragraphs: [
          "Identité et contact : nom, prénom, adresse email, téléphone.",
          "Plongée : date de naissance, numéro de licence, niveaux et brevets, date du certificat médical, préférences de matériel, compétences validées.",
          "Connexion : identifiant technique fourni par Google, Microsoft ou Apple si vous choisissez ces méthodes, et la liste de vos appareils connectés (type d'appareil et de navigateur, sans adresse IP).",
        ],
      },
      {
        title: "Connexion avec Google, Microsoft ou Apple",
        paragraphs: [
          "Si vous utilisez l'une de ces méthodes, nous recevons uniquement votre identifiant chez ce fournisseur, votre nom et votre adresse email. Nous n'accédons à aucune autre donnée de votre compte (emails, contacts, fichiers…).",
          "Avec « Masquer mon adresse email » d'Apple, nous recevons une adresse relais : nos emails vous parviennent via Apple.",
        ],
      },
      {
        title: "Finalités",
        paragraphs: [
          "Gérer votre adhésion et les activités de vos clubs (sorties, fosses, palanquées, formations), vérifier les prérogatives de plongée et vous envoyer les emails nécessaires au service (connexion, informations du club). Aucune publicité, aucune revente, aucun pistage.",
        ],
      },
      {
        title: "Qui y a accès ?",
        paragraphs: [
          "Les responsables des clubs dont vous êtes membre, selon leurs droits ; l'exploitant de la plateforme pour l'assistance. Les données sont hébergées [à compléter : pays / hébergeur].",
        ],
      },
      {
        title: "Durée de conservation",
        paragraphs: [
          "Les données sont conservées tant que vous êtes membre d'un club utilisant Dive Manager. Les sessions de connexion expirent après 6 mois d'inactivité ; les demandes d'accès traitées sont supprimées après 90 jours.",
        ],
      },
      {
        title: "Vos droits",
        paragraphs: [
          "Vous pouvez accéder à vos données, les rectifier ou demander leur suppression, et retirer une méthode de connexion depuis votre profil. Pour toute demande : [à compléter : email de contact]. Vous pouvez aussi saisir la CNIL (www.cnil.fr).",
        ],
      },
      {
        title: "Cookies",
        paragraphs: [
          "Dive Manager n'utilise aucun cookie publicitaire ni outil de mesure d'audience. Votre session est conservée dans le stockage local de votre navigateur.",
        ],
      },
    ],
  },

  sso: {
    finishing: "Connexion en cours…",
    failed: "Cette connexion a expiré. Recommencez depuis la page de connexion.",
    checkEmailTitle: "Vérifiez vos emails",
    checkEmail:
      "Un compte Dive Manager utilise déjà cette adresse. Pour des raisons de sécurité, nous vous avons envoyé un email : cliquez sur le lien pour confirmer et vous connecter.",
    unknownTitle: "Compte introuvable",
    unknown:
      "Aucun compte Dive Manager n'est associé à cette connexion. Demandez à votre club de vous ajouter, ou connectez-vous avec l'adresse email que votre club connaît.",
  },

  magicLink: {
    title: "Connexion en cours…",
    invalid:
      "Ce lien de connexion est invalide ou expiré. Demandez-en un nouveau.",
    backToLogin: "Retour à la connexion",
  },

  home: {
    hello: (name: string) => `Bonjour ${name}`,
    myClubs: "Mes clubs",
    noClub:
      "Vous n'êtes membre d'aucun club pour le moment. Un administrateur de votre club doit vous ajouter.",
    open: "Ouvrir",
    admin: "Administrateur",
  },

  club: {
    switch: "Changer de club",
    tabs: {
      members: "Membres",
      competencies: "Compétences",
      audit: "Journal",
    },
    members: {
      title: "Membres",
      role: "Rôle",
      impersonate: "Voir comme",
      suspend: "Suspendre",
      reactivate: "Réactiver",
      changeRole: "Changer le rôle",
      addTitle: "Ajouter un membre",
      addHint:
        "Si l'email correspond à un compte existant, ce compte rejoint le club. Sans email, la personne pourra être rattachée plus tard.",
      emailOptional: "Adresse email (facultative)",
      birthdateOptional: "Date de naissance (facultative)",
      add: "Ajouter",
      created: "Membre créé. Un email de bienvenue a été envoyé s'il a une adresse.",
      addedExisting: "Compte existant ajouté au club.",
      diffsTitle: "Valeurs différentes déjà enregistrées (non modifiées) :",
      diffField: {
        name: "Nom",
        birthdate: "Date de naissance",
        phone: "Téléphone",
        licence_number: "N° de licence",
      },
      diffKept: "conservé",
      diffEntered: "saisi :",
      diffHidden: "différente de la valeur enregistrée (conservée)",
      checkQuestion: (name: string) =>
        `Un compte existe déjà avec cet email, au nom de ${name}. Rien n'a été enregistré : s'agit-il bien de la même personne ? Sinon, corrigez l'adresse.`,
      checkConfirm: "Oui, c'est la même personne — l'ajouter",
    },
    audit: {
      title: "Journal d'activité",
      by: "par",
    },
  },

  platform: {
    title: "Plateforme",
    openClub: "Ouvrir",
    clubs: "Clubs",
    createClub: "Créer un club",
    clubName: "Nom du club",
    clubSlug: "Identifiant (URL)",
    addClubAdmin: "Ajouter un administrateur",
    clubAdminAdded: "Administrateur ajouté, email de bienvenue envoyé.",
    suspend: "Suspendre",
    reactivate: "Réactiver",
    globalAdmins: "Administrateurs globaux",
    addGlobalAdmin: "Ajouter un administrateur global",
    removeGlobalAdmin: "Révoquer",
    userStatusTitle: "Désactiver un compte",
    userStatusHint:
      "Saisissez l'email d'un compte à désactiver (ses sessions seront révoquées) ou à réactiver.",
    disable: "Désactiver",
    enable: "Réactiver",
  },

  loginMethods: {
    title: "Méthodes de connexion",
    devicesTitle: "Mes appareils",
    emailLink: "Lien par email",
    noEmail: "Aucune adresse — ajoutez-en une ci-dessous",
    lastUsed: (date: string) => `utilisé le ${date}`,
    remove: "Retirer",
    removed: "Méthode de connexion retirée.",
    add: (provider: string) => `Ajouter ${provider}`,
    linked: (provider: string) => `${provider} est maintenant associé à votre compte.`,
    alreadyUsed:
      "Ce compte est déjà associé à un autre compte Dive Manager (ou vous en avez déjà un de ce type : retirez-le d'abord).",
    changeEmail: "Changer d'adresse email",
    setEmail: "Ajouter une adresse email",
    sendConfirmation: "Envoyer un lien de confirmation",
    emailConfirmationSent: (email: string) =>
      `Un lien de confirmation a été envoyé à ${email}. L'adresse changera quand vous aurez cliqué dessus.`,
    unknownDevice: "Appareil inconnu",
    thisDevice: "cet appareil",
    lastSeen: (date: string) => `dernière activité le ${date}`,
    logoutDevice: "Déconnecter",
  },

  profile: {
    title: "Mon profil",
    editIdentity: "Modifier",
    logout: "Se déconnecter",
    logoutAll: "Se déconnecter de tous mes appareils",
  },

  diving: {
    incompleteBanner:
      "Votre profil plongée est incomplet : renseignez votre date de naissance pour connaître vos prérogatives.",
    incompleteBannerCta: "Compléter mon profil",
    incompleteMember:
      "Profil incomplet : la date de naissance est nécessaire pour calculer les prérogatives.",
    sectionProfile: "Profil plongée",
    sectionCertifications: "Certifications et qualifications",
    sectionPrerogatives: "Mes prérogatives",
    sectionPrerogativesMember: "Prérogatives",
    birthdate: "Date de naissance",
    phone: "Téléphone",
    licenceNumber: "Numéro de licence",
    instructorNumber: "Numéro d'encadrant (E2/E3/E4)",
    medical: "Certificat médical (CACI)",
    medicalValidUntil: "CACI valable jusqu'au",
    medicalManagedByClub: "Renseigné par les administrateurs du club.",
    medicalExpired: "CACI expiré",
    medicalMissing: "CACI non renseigné",
    medicalSaved: "Date de CACI enregistrée.",
    gear: "Mon matériel",
    gearRegulator: "Détendeur",
    gearSecondRegulator: "Second détendeur",
    gearStab: "Stab",
    gearStabSize: "Taille de stab",
    gearNitrox: "Équipement nitrox",
    handisub: "Plongée Handisub (situation de handicap)",
    preparing: "Certification en préparation",
    preparingNone: "Aucune",
    preparingIssues: "Points à vérifier pour cette préparation :",
    saved: "Profil enregistré.",

    certificationsManagedByClub:
      "Vos certifications sont enregistrées par les encadrants et administrateurs de votre club.",
    obtainedOn: "Obtenue le (facultatif)",
    selectCertification: "Choisir une certification…",
    coveredBySelection: "Acquis via la sélection",
    willBeRemoved: "Sera retirée",
    certificationsSaved: "Certifications mises à jour.",
    remove: "Retirer",
    provenance: {
      declared: "Déclarée",
      recorded: "Enregistrée",
      imported: "Importée",
      validated: "Validée",
    } as Record<string, string>,

    aptitudes: "Aptitudes en plongée",
    aptitudeKind: {
      guided: "encadré",
      autonomous: "autonome",
      pesh: "PESH",
    } as Record<string, string>,
    noAptitude: "Aucune aptitude dérivée pour le moment.",
    depth: (m: number) => `${m} m`,
    restrictions: "Restrictions applicables",
    supervision: "Encadrement",
    gpYes: "Guide de palanquée (exploration)",
    teachingGrade: (grade: string) => `Enseignement : ${grade}`,
    noSupervision: "Pas de prérogative d'encadrement.",
    dp: "Direction de plongée",
    dpPool: "Piscine / fosse ≤ 6 m",
    dpExploration: "Exploration air/nitrox",
    dpTeaching: "Enseignement",
    dpTrimixShallow: "Trimix zone limitée (ens. ≤ 40 m, explo ≤ 70 m)",
    dpTrimix: "Trimix toutes zones",
    dpMixtures:
      "Pour diriger des plongées aux mélanges, le PN-C ou PTH-120 est requis (détenu).",
    noDp: "Ne peut pas être directeur de plongée.",
    gas: "Gaz",
    maxO2: (pct: number) => `O₂ max : ${pct} %`,
    eligibleNext: "Certifications préparables",
    noEligible: "Rien à proposer pour le moment.",
    age: (a: number) => `${a} ans`,
    regulationNote: (snapshot: string) =>
      `Prérogatives calculées selon : ${snapshot}. Elles ne remplacent pas le jugement du directeur de plongée.`,

    stamp: "Tampon d'encadrant",
    stampHint:
      "PNG ou JPEG, 512 Ko maximum. Réservé aux encadrants E2 et plus (certification validée).",
    stampUpload: "Téléverser un tampon",
    stampReplace: "Remplacer le tampon",
    stampDelete: "Supprimer le tampon",
    stampSaved: "Tampon enregistré.",
    stampTooBig: "L'image dépasse 512 Ko.",

    ffessm: {
      title: "Importer ma licence FFESSM",
      hint: "Prenez en photo le QR code de votre licence dématérialisée, ou collez son lien. Votre profil et vos brevets sont remplis automatiquement depuis la source officielle FFESSM.",
      scan: "Scanner le QR code",
      scanning: "Lecture du QR code…",
      scanFailed:
        "QR code illisible. Cadrez-le en plein écran et réessayez, ou collez le lien.",
      urlLabel: "Lien du QR code de la licence",
      urlPlaceholder: "https://infolicencie.ffessm.fr/Home/InfoLicence?number=…&key=…",
      import: "Importer la licence",
      importing: "Import en cours…",
      resultFor: (name: string) => `Licence de ${name} importée.`,
      imported: (n: number) =>
        n === 1 ? "1 brevet importé." : `${n} brevets importés.`,
      skipped: (codes: string) =>
        `Conservés (déjà validés par le club) : ${codes}.`,
      unmapped: (labels: string) =>
        `Non reconnus, à saisir manuellement : ${labels}.`,
      empty: "Aucun brevet trouvé sur cette licence.",
    },

    memberProfileTitle: "Fiche plongeur",
    memberOpenProfile: "Fiche plongeur",
    recordTitle: "Enregistrer une certification",
    recordValidated: "Validée par le club (source officielle vérifiée)",
    record: "Enregistrer",
    trainingStatus: "Stagiaire pédagogique MF1",
    trainingStatusHint:
      "Livret pédagogique en cours de validité : confère E2 (enseignement uniquement, jamais DP).",
    trainingStatusUntil: "Livret valable jusqu'au",
    trainingStatusSet: "Définir / renouveler",
    trainingStatusRemove: "Retirer le statut",
    badgeGp: "GP",
    badgeRifap: "RIFAP",
    quickRifap: "RIFAP validé aujourd'hui",
  },

  impersonation: {
    banner: (name: string) => `Vous agissez en tant que ${name}`,
    stop: "Reprendre ma session",
  },

  competencies: {
    myTitle: "Mes compétences",
    stages: {
      vu: "Vu",
      a_perfectionner: "À perfectionner",
      valide: "Validé",
      valide_mer: "Validé en mer",
      not_started: "Non commencé",
    } as Record<string, string>,
    stageOf: (code: string, sea: boolean) =>
      code === "valide" && sea ? "Validé en mer" : (t.competencies.stages[code] ?? code),
    legend: "Étapes de validation",
    seaRule:
      "Au-dessus du Niveau 1, la validation finale se fait obligatoirement en mer.",
    progress: (validated: number, total: number) =>
      `${validated}/${total} validés`,
    progressDetail: (v: number, p: number, n: number) =>
      `${v} validés • ${p} en cours • ${n} à faire`,
    preparing: (label: string) => `En préparation : ${label}`,
    validatedBy: (name: string) => `par ${name}`,
    validatedOn: (date: string) => `le ${date}`,
    downloadPdf: "Télécharger le document (PDF)",
    emptyNoPreparation:
      "Aucun niveau en préparation. Contactez un encadrant ou un administrateur de votre club pour configurer votre formation.",
    emptyNoTree:
      "Le référentiel de ce niveau n'est pas encore défini.",
    explainerTitle: "Comment ça marche ?",
    explainer:
      "Vos acquis sont validés par les encadrants au fil des séances (piscine, fosse, mer). Chaque acquis passe par les étapes Vu → À perfectionner → Validé. Quand tout est validé, vous pouvez passer votre niveau !",

    studentsTitle: "Élèves en préparation",
    studentsLevel: "Niveau préparé",
    noStudents: "Aucun élève ne prépare ce niveau dans ce club.",
    openSheet: "Voir la progression",
    sheetTitle: (name: string) => `Compétences — ${name}`,
    requiredGrade: (grade: string) =>
      `Validation réservée aux encadrants ${grade} et plus.`,
    validateConfirmTitle: "Confirmer la validation",
    validateConfirm: (skill: string, stage: string) =>
      `Passer « ${skill} » à l'étape « ${stage} » ?`,
    validateIrreversible:
      "Attention : une étape ne peut pas revenir en arrière (seul un administrateur global peut corriger).",
    validateSeaReminder:
      "Ce niveau se valide en mer : ne validez cette étape finale qu'en milieu naturel.",
    notes: "Notes (facultatif)",
    validatedToast: "Validation enregistrée.",
    deleteValidation: "Supprimer la validation",
    deleteConfirm: (skill: string) =>
      `Supprimer la validation de « ${skill} » ? L'historique est conservé.`,

    adminTitle: "Référentiel de compétences",
    adminHint:
      "Référentiel global de la plateforme : niveaux → domaines → modules → acquis. Les étapes de validation sont fixes (Vu → À perfectionner → Validé).",
    level: "Niveau",
    domain: "Domaine",
    module: "Module",
    skill: "Acquis",
    addDomain: "Ajouter un domaine",
    addModule: "Ajouter un module",
    addSkill: "Ajouter un acquis",
    skillDescription: "Description (aide affichée aux encadrants)",
    name: "Nom",
    edit: "Modifier",
    delete: "Supprimer",
    deleteImpact: (n: number) =>
      n === 0
        ? "Aucune validation de membre ne sera supprimée."
        : n === 1
          ? "1 validation de membre sera définitivement supprimée !"
          : `${n} validations de membres seront définitivement supprimées !`,
    deleteTreeConfirm: (kind: string, name: string) =>
      `Supprimer ${kind} « ${name} » et tout son contenu ?`,

    documentsTitle: "Documents de niveau",
    documentsHint:
      "Un template PDF par niveau ; placez les cases de tampon sur le document, elles seront remplies avec les validations finales.",
    upload: "Téléverser un PDF",
    replace: "Remplacer le PDF",
    deleteDocument: "Supprimer le document",
    deleteDocumentConfirm: (level: string) =>
      `Supprimer le document du niveau ${level} ? Les positions de tampon seront supprimées avec lui.`,
    pages: (n: number) => (n === 1 ? "1 page" : `${n} pages`),
    noDocument: "Aucun document",
    positionsPlaced: (n: number) => `Placés (${n})`,
    positionsUnplaced: (n: number) => `À placer (${n})`,
    place: "Placer",
    placing: (skill: string) =>
      `Mode placement : dessinez un rectangle sur le PDF pour « ${skill} ».`,
    positionSaved: "Position enregistrée.",
    positionDeleted: "Position supprimée.",
    outOfRange: "Hors page",
    zoom: "Zoom",
    page: (p: number, total: number) => `Page ${p} / ${total}`,
    editorHelp:
      "Glissez pour déplacer une case • poignées d'angle pour redimensionner • éditeur utilisable à la souris (ordinateur).",
    documentUploaded: "Document téléversé.",
  },
};
