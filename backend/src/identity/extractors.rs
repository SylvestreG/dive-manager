use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::AppError;
use crate::identity::services::sessions::{self, AuthSession};
use crate::AppState;

/// Extracts and authenticates the bearer token. Handlers that only need an
/// authenticated user (no club context) take this directly.
pub struct CurrentUser(pub AuthSession);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;
        let auth = sessions::authenticate(&state.db, &state.config, token).await?;
        Ok(CurrentUser(auth))
    }
}

fn bearer_token(parts: &Parts) -> Result<&str, AppError> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AppError::Unauthorized("missing bearer token".to_string()))
}

/// Best-effort client IP for rate limiting. `X-Forwarded-For` is honored
/// ONLY when `server.trust_proxy` is enabled (a directly exposed server must
/// not let clients forge IPs to bypass per-IP limits); otherwise the socket
/// peer address is used, with a shared bucket as last resort.
pub fn client_ip(parts: &Parts, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(forwarded) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(first) = forwarded.split(',').next() {
                let first = first.trim();
                if !first.is_empty() {
                    return first.to_string();
                }
            }
        }
    }
    parts
        .extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Extractor variant of [`client_ip`] for handlers.
pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(client_ip(parts, state.config.server.trust_proxy)))
    }
}
