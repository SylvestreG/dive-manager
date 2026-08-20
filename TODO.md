# TODO

## Milestone

**USI full cutover in September 2026** (go/no-go ~2026-09-20; fallback = keep
using Fosse). Scope and checklist: `docs/design/migration-strategy.md`.

## Decisions recorded 2026-08-17

- ADR-0002 (shared tables + `club_id`) and ADR-0004 (auth; 12 h login magic
  links) accepted.
- September = **full cutover**, not import-only.
- **No security hotfix on Fosse** — residual risk accepted until cutover.
- **Dive Manager sends emails** (SMTP), no copy-paste workflow.
- "Encadrant ⇒ DP tools on next sortie" = accident, not reproduced.
- Bottle-optimization formula: keep Fosse's for now, isolate it, fix later.

## Next (in order)

- [x] **FFESSM domain lib**: research sourced from the latest MFT + Code du
      Sport 2026 (`docs/analysis/ffessm-levels.md`); scope validated (4/4);
      `backend/ffessm` crate **implemented, reviewed, committed** (dc3e13b).
- [x] **`users` module** (renamed from `members`: profile is USER-GLOBAL,
      product decision 2026-08-17) — design approved
      (`docs/design/users-module.md`), **implemented + tested** (backend
      100 tests green: tenancy isolation/IDOR, provenance matrix, derived
      personas over HTTP, stamp gating, impersonation/authorization matrix;
      frontend: profile page with full prerogatives, red incomplete banner,
      member sheet, badges on the members list). Senior review done
      (2026-08-17); all approved findings fixed (highlights: /me writes
      refused while impersonating, has_stamp without blob load, DP trimix
      limited zone exposed, FK indexes, CACI read moved into the txn,
      preparation cleared on obtention). Awaiting product-owner validation
      before commit.

- [x] Frontend stack decided — ADR-0005: React + TS + Vite PWA + Capacitor;
      store v1 = member app, base modules suffice (product owner, 2026-08-17)
- [ ] **Product owner, this week (critical path, only you can do these):**
  - [ ] Apple Developer account — decide individual vs organization
        (organization needs a D-U-N-S number: 1–2 weeks lead time)
  - [ ] Google Play developer account — **organization** account (a recent
        personal account forces 14 days of closed testing with 12 testers)
  - [x] SMTP: **reuse Fosse's OVH account** (ssl0.ovh.net, USI sender) as the
        platform sender for now; becomes USI's club SMTP config later.
        Verify SPF/DKIM for the sending domain before the first invitation wave
  - [x] Hosting: **same VPS as Fosse**. Consequence: coexistence required —
        own PostgreSQL database on the shared instance, own port, reverse
        proxy routes per app; Fosse must remain untouched and running
- [x] GO given; workspace + CI + foundation (`identity` + `tenancy`)
      **implemented and tested** (2026-08-17). Not yet committed to git.
- [x] Senior review done; all HIGH/MEDIUM + L2–L9 findings **fixed and
      tested** (28 tests green). Remaining from review: NICE-TO-HAVE items
      (injectable clock, httpOnly cookie option, tracing request ids) + L1
      (stable error-code doc) — tracked, not blocking.
- [x] First commit done (c059d8a)
- [x] Security review items closed (2026-08-17): login-methods oracle
      formally accepted + documented (foundation-module.md); courtesy email on
      direct add → `notifications` module scope; public-auth 401s no longer
      trigger the global session purge
- [x] Frontend v1 (React+Vite+Capacitor, Abysse design system): login (magic
      link + password), invitation acceptance, home, platform admin, club
      tabs (members/invitations/roles/modules/audit), impersonation banner.
      End-to-end smoke verified (magic-link flow through the Vite proxy).
- [ ] **Product owner: install Xcode + CocoaPods** to run the iPhone
      simulator (`DEVELOPMENT.md` §frontend), then `npm run ios`
- [ ] Next slice: OIDC Google login (schema ready) — needs the Google Cloud
      OAuth client (product owner)
- [x] Done (2026-08-18, chained agents):
  - [x] Certification checkbox tree on the fiche membre (`ffessm::covered_by`
        + catalog endpoint)
  - [x] Instructor section (numéro + tampon) gated on a derived ffessm
        teaching grade
  - [x] ADR-0007 implemented: seed = Administrateur+Membre only, dev-data
        Encadrant roles removed (members reassigned to Membre), tests build
        staff personas via an explicit `create_role` helper
- [x] **`competencies` module** (2026-08-20) — design approved + implemented
      (docs/design/competencies-module.md, ADR-0008): platform-global tree,
      fixed stages (vu/à perfectionner/validé, "en mer" above N1), grade per
      level (E1<N2, E2<N3, E3<N4, E4+), user-global validations with
      append-only history, level documents + stamp editor + flattened PDF.
      Backend 106 lib tests green (30 new); frontend 15 tests green (4 new
      + fixed the Node-26 localStorage/vitest breakage); Modules tab removed
      from the club UI (registry kept server-side). **Awaiting
      `dive-manager-review` + product-owner validation before commit.**
      Deferred (per design): stamp image on the PDF, "eligible for delivery"
      indicator, sortie group builder (needs `activities`).
- [ ] Importer skeleton + first real-data reconciliation report (includes the
      `diving_level` comma-blob → certification codes mapping audit, CSV
      bulk entry of CACI/RIFAP dates — product decision 2026-08-17 — and the
      Fosse competencies mapping fixed in competencies-module.md §10: stages →
      fixed ladder, min_validator_level dropped, legacy flat table checked
      then dropped)
- [ ] Then `activities` → `registrations` → `palanquees` → `imports` →
      `notifications`, per `MODULES.md` dependencies

## Open questions for the product owner

- Which Fosse features are actually used vs dead weight? (several orphan pages
  and no-op endpoints found — see `docs/analysis/`)
- Expected scale after USI: how many clubs / users in year one?
- Production hosting for Dive Manager (same VPS as Fosse? new one?) and the
  SMTP account to send from (deliverability: SPF/DKIM).
