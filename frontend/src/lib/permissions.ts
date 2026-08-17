// Club permission catalogue — the frontend's single source of truth, mirrors
// the backend `Permission` enum. Labels live in the i18n dictionary.
export const CLUB_PERMISSIONS = [
  "club.manage",
  "club.members.view",
  "club.members.manage",
  "club.roles.manage",
  "club.invitations.manage",
  "club.impersonate",
] as const;
