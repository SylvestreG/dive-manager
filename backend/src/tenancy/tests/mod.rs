//! Tenancy integration tests — real PostgreSQL via
//! [`crate::test_support::TestApp`], one file per theme.

mod authorization;
mod impersonation;
mod invitations;
mod isolation;
mod platform;
