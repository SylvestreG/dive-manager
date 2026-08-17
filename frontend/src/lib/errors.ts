// French translation of API errors. Frontend rule (docs/design/frontend.md):
// every error must be visibly surfaced to the user — this module provides the
// message, the toast system guarantees the display.
import { t } from "../i18n/fr";
import { ApiError } from "./api";

/** Known backend messages → French. Fallback: show the backend message for
 * 4xx (it is written for humans), generic message otherwise. */
const KNOWN_MESSAGES: Record<string, string> = {
  "password must be between 8 and 128 characters":
    "Le mot de passe doit contenir entre 8 et 128 caractères.",
  "invalid credentials": "Identifiants invalides.",
  "invalid or expired session": "Session expirée, reconnectez-vous.",
  "invalid or expired login link": "Lien de connexion invalide ou expiré.",
  "resource already exists": "Cet élément existe déjà.",
  "this user is already a member of the club":
    "Cette personne est déjà membre du club.",
  "already a member of this club": "Cette personne est déjà membre du club.",
  "no account with this email — send an invitation instead":
    "Aucun compte avec cet email — envoyez une invitation.",
  "cannot remove the club's last administrator":
    "Impossible de retirer le dernier administrateur du club.",
  "cannot remove the last global administrator":
    "Impossible de retirer le dernier administrateur global.",
  "cannot revoke your own global-admin status":
    "Vous ne pouvez pas révoquer votre propre statut d'administrateur global.",
  "cannot impersonate yourself": "Vous ne pouvez pas vous impersonnifier.",
  "cannot impersonate a global administrator":
    "Impossible d'impersonnifier un administrateur global.",
  "action not allowed while impersonating":
    "Action impossible pendant une impersonation.",
  "invitation not found or expired": "Invitation introuvable ou expirée.",
  "this account is disabled": "Ce compte est désactivé.",
  "user is not a member of this club":
    "Cette personne n'est pas membre du club.",
  "not a member of this club": "Vous n'êtes pas membre de ce club.",
};

export function translateApiError(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 429) return t.login.tooManyAttempts;
    const known = KNOWN_MESSAGES[error.message];
    if (known) return known;
    if (error.status >= 400 && error.status < 500 && error.message) {
      return error.message;
    }
  }
  return t.common.error;
}
