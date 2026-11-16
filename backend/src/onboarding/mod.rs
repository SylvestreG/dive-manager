//! Member onboarding (docs/design/auth-onboarding.md, ADR-0009): how people
//! enter the platform. Accounts are admin-curated — manual creation today;
//! CSV import and access-request review in later slices.
//!
//! Depends on `identity` (users), `tenancy` (clubs, memberships, roles) and
//! `users` (profile gap filling); never writes their tables directly.

pub mod handlers;
pub mod services;

#[cfg(test)]
mod tests;
