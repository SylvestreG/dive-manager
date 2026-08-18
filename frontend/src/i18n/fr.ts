// French UI copy — the single dictionary (translation-layer embryo).
// Components never hardcode user-facing strings.
export const t = {
  appName: "Dive Manager",

  nav: {
    home: "Accueil",
    club: "Mon club",
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
    password: "Mot de passe",
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
    continue: "Continuer",
    magicLinkSent:
      "Si cette adresse est connue, un lien de connexion vient d'être envoyé par email. Il est valable 12 heures.",
    passwordAction: "Se connecter",
    forgotPassword: "Mot de passe oublié ? Recevoir un lien par email",
    changeEmail: "Changer d'adresse",
    invalidCredentials: "Identifiants invalides.",
    tooManyAttempts: "Trop de tentatives, réessayez dans quelques minutes.",
  },

  magicLink: {
    title: "Connexion en cours…",
    invalid:
      "Ce lien de connexion est invalide ou expiré. Demandez-en un nouveau.",
    backToLogin: "Retour à la connexion",
  },

  invitation: {
    title: "Invitation",
    joinClub: (club: string) => `Vous êtes invité·e à rejoindre ${club}`,
    joinPlatform: "Vous êtes invité·e à administrer Dive Manager",
    role: "Rôle",
    optionalPassword:
      "Choisir un mot de passe (facultatif — le lien de connexion par email reste disponible)",
    accept: "Accepter l'invitation",
    invalid: "Cette invitation est invalide, expirée ou déjà utilisée.",
  },

  home: {
    hello: (name: string) => `Bonjour ${name}`,
    myClubs: "Mes clubs",
    noClub:
      "Vous n'êtes membre d'aucun club pour le moment. Un administrateur doit vous inviter.",
    open: "Ouvrir",
    admin: "Administrateur",
  },

  club: {
    switch: "Changer de club",
    tabs: {
      members: "Membres",
      invitations: "Invitations",
      modules: "Modules",
      audit: "Journal",
    },
    members: {
      title: "Membres",
      role: "Rôle",
      impersonate: "Voir comme",
      suspend: "Suspendre",
      reactivate: "Réactiver",
      changeRole: "Changer le rôle",
      addExisting: "Ajouter un membre existant",
      addExistingHint:
        "Le compte doit déjà exister sur Dive Manager. Pour une nouvelle personne, passez par une invitation.",
      add: "Ajouter",
      added: "Membre ajouté.",
    },
    invitations: {
      title: "Invitations en attente",
      invite: "Inviter un membre",
      resend: "Renvoyer",
      revoke: "Révoquer",
      expires: "Expire le",
      sent: "Invitation envoyée.",
    },
    modules: {
      title: "Modules du club",
      hint: "Activez les fonctionnalités utilisées par votre club.",
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
    inviteAdmin: "Inviter un administrateur",
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

  profile: {
    title: "Mon profil",
    setPassword: "Définir un mot de passe",
    changePassword: "Changer le mot de passe",
    newPassword: "Nouveau mot de passe",
    passwordHint:
      "8 caractères minimum. Facultatif : le lien de connexion par email reste toujours disponible.",
    changePasswordHint:
      "8 caractères minimum. Le nouveau mot de passe remplace l'ancien.",
    passwordSaved: "Mot de passe enregistré.",
    passwordTooShort: "Le mot de passe doit contenir au moins 8 caractères.",
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

  modules: {
    members: "Membres",
    activities: "Activités (fosses & sorties)",
    registrations: "Inscriptions & questionnaires",
    palanquees: "Palanquées",
    competencies: "Compétences",
    imports: "Imports CSV",
    notifications: "Notifications & emails",
  } as Record<string, string>,
};
