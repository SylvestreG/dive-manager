# Frontend design

Status: v1 (2026-08-17). Implements ADR-0005. Scope of this slice: a working
mobile-first web app + Capacitor iOS shell covering the foundation module
(auth, clubs, memberships, roles, invitations, impersonation, platform admin).

## Stack decision (recap of ADR-0005)

| Option | Web | iOS/Android | Sharing | Verdict |
|---|---|---|---|---|
| **React + TS + Vite PWA, wrapped with Capacitor** | native DOM, fast | store apps from the same bundle | 100 % | **chosen** |
| Flutter | heavy CanvasKit web, weak for dense forms | good | 100 % | rejected (web quality, timeline) |
| React Native + Expo | react-native-web compromises desktop admin UIs | good | ~90 % | rejected (web quality) |
| Rust SSR + HTMX | light | none native | n/a | rejected (interactive builders to come) |

Libraries (deliberately few): `react-router` (routing), `@tanstack/react-query`
(server state — Fosse's manual `loadData()` waterfalls are the
counter-example), Tailwind CSS v4 (utility styling over design tokens).
No axios (typed `fetch` client), no state library (token + react-query
suffice), **no webfonts** (system font stack — poor-network requirement).

## Visual identity — "Abysse"

Original identity; FFESSM sites are the reference for *usability and
information architecture only* (dense but legible lists, clear member/club
hierarchy, sober federation feel).

Tokens (CSS custom properties, mapped into Tailwind):

- **Primary "abysse"**: #0B5B7E (600), #094A67 (700), #E8F3F8 (50) — deep
  ocean blue, used for actions and active navigation.
- **Neutrals**: slate scale on a #F4F7F9 background; text #14252E.
- **Semantic**: success #157F3D · warning #A8621A · danger #B3261E, each with
  a 50-tint for surfaces.
- **Typography**: system stack (`-apple-system, Segoe UI, Roboto…`), base
  16 px, titles 600 weight; no font downloads.
- **Shape**: 12 px card radius, 8 px control radius, 1 px hairline borders
  (#D8E2E8), soft single shadow on cards only.
- **Touch**: minimum 44 px tap targets; primary actions full-width on phones.

Light theme only in v1 (dark later; Fosse's half-migrated theming is the
counter-example — one finished theme beats two broken ones).

## Information architecture

- **Phone (default)**: bottom tab bar — `Accueil`, `Mon club`, `Profil`
  (+ `Plateforme` for global admins). Club sections as horizontally
  scrollable segmented tabs under the club header.
- **Desktop/tablet (≥ 768 px)**: the tab bar becomes a left sidebar; content
  max-width 960 px.
- **Impersonation**: unmissable full-width amber banner above everything +
  ring around the viewport (kept from Fosse — proven pattern), with the
  target's name and a "Reprendre ma session" action.
- **States**: every list renders exactly one of loading skeleton / error with
  retry / explicit empty state / data. No silent failures (Fosse's swallowed
  `catch {}` is the counter-example).
- **Error-visibility rule (product owner, 2026-08-17): every error MUST be
  visibly surfaced to the user.** Enforced by construction: the react-query
  `MutationCache.onError` fires a global error toast for any failed write —
  pages cannot forget to handle errors. Forms that display the error inline
  (next to the field) opt out of the duplicate toast with
  `meta: { inlineError: true }`. API messages are translated to French in
  `lib/errors.ts` (`translateApiError`); unknown 4xx messages are shown as-is,
  5xx fall back to a generic message.

## Screens (foundation scope)

| Route | Content |
|---|---|
| `/login` | email-first: « Recevoir un lien de connexion » (default) or password toggle; generic confirmations (no enumeration) |
| `/auth/ml/:token` | consumes the magic link, opens session |
| `/invitations/:token` | invitation preview (club, role) + accept, optional password |
| `/` | greeting, club membership cards (role badge), quick actions |
| `/platform` | clubs (create, suspend, invite admin), global admins, user enable/disable |
| `/clubs/:slug` | club header + tabs: Membres · Invitations · Rôles · Modules · Journal (tabs filtered by permissions) |
| `/profile` | identity, set password, logout / logout everywhere |

French UI copy lives in `src/i18n/fr.ts` (single dictionary — the embryo of
the translation layer; never hardcoded in components).

## API access

Typed fetch client (`src/lib/api.ts`): bearer token from localStorage,
JSON errors surfaced as `{code, message}`, 401 → purge session + redirect to
login (except during impersonation-stop flows). Base URL: `VITE_API_URL`
(defaults to same-origin `/api/v1`; the Vite dev server proxies to the
backend; the iOS build points at `http://localhost:8081` — the simulator
shares the host network).

## iOS (Capacitor)

Same bundle wrapped by Capacitor. Dev flow: `npm run build && npx cap sync
ios && npx cap open ios` → run on the iPhone simulator. Cleartext http is
allowed in the dev Capacitor config only; a production build must use https.
