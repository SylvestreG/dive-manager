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
- [ ] Product owner reviews; first git commit
- [ ] Next slice: OIDC Google login (schema ready) — needs the Google Cloud
      OAuth client (product owner)
- [ ] `members` design + implementation (audit real `diving_level` data first)
- [ ] Frontend PWA/Capacitor skeleton (de-risk the iOS/Android build chain)
- [ ] Importer skeleton + first real-data reconciliation report
- [ ] Then `activities` → `registrations` → `palanquees` → `competencies` →
      `imports` → `notifications`, per `MODULES.md` dependencies

## Open questions for the product owner

- Which Fosse features are actually used vs dead weight? (several orphan pages
  and no-op endpoints found — see `docs/analysis/`)
- Expected scale after USI: how many clubs / users in year one?
- Production hosting for Dive Manager (same VPS as Fosse? new one?) and the
  SMTP account to send from (deliverability: SPF/DKIM).
