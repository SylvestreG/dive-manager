//! Identity integration tests — real PostgreSQL via
//! [`crate::test_support::TestApp`], one file per theme.

mod bootstrap;
mod identity;
mod magic_links;
mod passwords;
mod sessions;
