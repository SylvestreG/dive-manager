//! FFESSM licence import: scan a member's dematerialized-licence QR code (or
//! paste its URL) and fill the profile from `infolicencie.ffessm.fr`.
//!
//! Layering: [`parse`] is pure (HTML → [`parse::LicencePreview`]); [`client`]
//! does the outbound fetch and target resolution. The self-service import
//! flow lives in the `users` module (`users::services::licence_import`); the
//! brevet-title → certification mapping lives in the pure `ffessm` crate.

pub mod client;
pub mod parse;
