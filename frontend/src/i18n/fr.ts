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
      roles: "Rôles",
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
    roles: {
      title: "Rôles et permissions",
      adminImplicit: "Toutes les permissions (implicite)",
      savePermissions: "Enregistrer les permissions",
      saved: "Permissions mises à jour.",
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

  impersonation: {
    banner: (name: string) => `Vous agissez en tant que ${name}`,
    stop: "Reprendre ma session",
  },

  permissions: {
    "club.manage": "Gérer le club",
    "club.members.view": "Voir les membres",
    "club.members.manage": "Gérer les membres",
    "club.roles.manage": "Gérer les rôles",
    "club.invitations.manage": "Gérer les invitations",
    "club.impersonate": "Voir comme un membre",
  } as Record<string, string>,

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
