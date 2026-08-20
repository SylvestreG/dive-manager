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
  "member not found": "Membre introuvable dans ce club.",
  "certification not found": "Certification introuvable.",
  "certification already recorded with a stronger provenance":
    "Cette certification est déjà enregistrée avec une provenance plus forte.",
  "invalid obtained_on date": "Date d'obtention invalide.",
  "invalid birthdate": "Date de naissance invalide.",
  "invalid stab size": "Taille de stab invalide.",
  "this title cannot be prepared in a club":
    "Ce titre ne se prépare pas en club.",
  "certification already held": "Cette certification est déjà détenue.",
  "certification already covered by held certifications":
    "Cette certification est déjà couverte par les certifications détenues.",
  "instructor number is reserved for instructors":
    "Le numéro d'encadrant est réservé aux encadrants (grade E1 à E4).",
  "stamp is reserved for E2+ instructors":
    "Le tampon est réservé aux encadrants E2 et plus (certification validée).",
  "stamp image exceeds 512 KiB": "L'image dépasse 512 Ko.",
  "stamp must be a PNG or JPEG image":
    "Le tampon doit être une image PNG ou JPEG.",
  "invalid base64 image": "Image invalide.",
  "set your birthdate before uploading a stamp":
    "Renseignez d'abord votre date de naissance.",
  "the member's birthdate must be set first":
    "La date de naissance du membre doit d'abord être renseignée.",
  "no stamp": "Aucun tampon enregistré.",
  "invalid licence link or number": "Lien ou numéro de licence invalide.",
  "licence not found or invalid key": "Licence introuvable ou clé invalide.",
  "licence holder does not match your account":
    "Cette licence ne correspond pas à votre compte (nom ou date de naissance différents). Vérifiez le prénom et le nom de votre compte en haut de cette page.",
  "first and last names cannot be empty":
    "Le prénom et le nom sont obligatoires.",
  "name too long": "Nom trop long.",
};

export function translateApiError(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 429) return t.login.tooManyAttempts;
    const known = KNOWN_MESSAGES[error.message];
    if (known) return known;
    if (error.message.startsWith("unknown certification: ")) {
      return `Certification inconnue : ${error.message.slice("unknown certification: ".length)}`;
    }
    if (error.status >= 400 && error.status < 500 && error.message) {
      return error.message;
    }
  }
  return t.common.error;
}
