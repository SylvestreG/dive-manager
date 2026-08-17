# Dive Manager

## Vision

Dive Manager is a complete rewrite and modernization of **Fosse**, a diving club
management application originally built for a single club (USI). Dive Manager is
multi-tenant from day one: it supports multiple diving clubs, users belonging to
multiple clubs, club administrators, and global (platform) administrators.

The goal is not to reproduce Fosse, but to keep and improve its useful
functionality on a cleaner, faster, more secure, more maintainable foundation.

## Product scope (target)

- Multiple clubs, each with its own configuration and enabled modules
- Users with memberships in one or more clubs
- Roles: global administrator, club administrator, club member (extensible)
- Modular functionality per club (members, sessions, dive trips, questionnaires,
  palanquées, competencies, emails, imports, documents, …) — final module
  boundaries derive from the Fosse analysis, not from a wish list

## Functional reference

Fosse lives at `../fosse` (sibling repository). It is the functional reference:
every Fosse feature is analyzed (business rules, data, permissions, edge cases,
bugs, debt) before being preserved, redesigned, or dropped. See `DOMAIN.md` for
the analysis and `MODULES.md` for the resulting module map.

Fosse stack, for reference: Rust (Axum 0.7, SeaORM 1, PostgreSQL), React +
TypeScript + Vite + Tailwind frontend, Google OAuth + magic links + passwords +
JWT for auth.

## Non-goals

- Microservices, Kubernetes, Redis, Kafka, Elasticsearch — no heavy
  infrastructure without a demonstrated requirement
- A generic plugin system — modules are internal to a modular monolith
- Cloning FFESSM assets or layouts — FFESSM is UX inspiration only

## Core requirements

- Backend entirely in stable Rust; modular monolith
- Single PostgreSQL database for all clubs; tenant isolation enforced by the
  backend on every request
- Excellent performance on modest infrastructure (small VPS)
- Security first: authz on every protected resource, no IDOR, no implicit
  privilege grants
- Mobile-first UX inspired by the French diving ecosystem (FFESSM terminology)
- English-only source code; project docs in English

## Working method

Development proceeds **module by module**:

Analyze → Design → Review → Implement → Test → Review → Document → Validate → next module.

A module is done only when it meets the Definition of Done (see `DEVELOPMENT.md`).

## Project documents

| File | Purpose |
|---|---|
| `PROJECT.md` | Vision, scope, requirements (this file) |
| `ARCHITECTURE.md` | Technical architecture and cross-cutting concerns |
| `DOMAIN.md` | Domain model and business rules (from Fosse analysis) |
| `MODULES.md` | Module map: responsibilities, dependencies, status |
| `DECISIONS.md` | Architecture decision records |
| `ROADMAP.md` | Module sequence and milestones |
| `TODO.md` | Current work items |
| `DEVELOPMENT.md` | Dev environment, conventions, Definition of Done |
