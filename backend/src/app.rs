use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::identity::handlers::{auth, me as me_password};
use crate::tenancy::handlers::{clubs, me, platform, public_invitations};
use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        // Public authentication endpoints.
        .route("/auth/magic-link", post(auth::request_magic_link))
        .route("/auth/magic-link/consume", post(auth::consume_magic_link))
        .route("/auth/login", post(auth::login_password))
        .route(
            "/auth/invitations/{token}",
            get(public_invitations::preview),
        )
        .route(
            "/auth/invitations/{token}/accept",
            post(public_invitations::accept),
        )
        // Authenticated session management.
        .route("/auth/logout", post(auth::logout))
        .route("/auth/logout-all", post(auth::logout_all))
        .route("/auth/stop-impersonation", post(auth::stop_impersonation))
        .route("/me", get(me::get_me))
        .route("/me/password", put(me_password::set_my_password))
        // Platform administration (real global admins only).
        .route(
            "/platform/clubs",
            get(platform::list_clubs).post(platform::create_club),
        )
        .route("/platform/clubs/{club_id}", put(platform::update_club))
        .route(
            "/platform/clubs/{club_id}/invitations",
            post(platform::invite_club_admin),
        )
        .route(
            "/platform/global-admins",
            get(platform::list_global_admins).post(platform::add_global_admin),
        )
        .route(
            "/platform/global-admins/{user_id}",
            delete(platform::remove_global_admin),
        )
        .route(
            "/platform/users/{user_id}/status",
            put(platform::set_user_status),
        )
        // Club-scoped endpoints (Tenant extractor enforces membership).
        .route(
            "/clubs/{club_slug}",
            get(clubs::get_club).put(clubs::update_club),
        )
        .route(
            "/clubs/{club_slug}/modules",
            get(clubs::list_modules).put(clubs::set_module),
        )
        .route("/clubs/{club_slug}/roles", get(clubs::list_roles))
        .route(
            "/clubs/{club_slug}/roles/{role_id}/permissions",
            put(clubs::set_role_permissions),
        )
        .route(
            "/clubs/{club_slug}/memberships",
            get(clubs::list_memberships),
        )
        .route(
            "/clubs/{club_slug}/memberships/{membership_id}",
            put(clubs::update_membership),
        )
        .route(
            "/clubs/{club_slug}/invitations",
            get(clubs::list_invitations).post(clubs::create_invitation),
        )
        .route(
            "/clubs/{club_slug}/invitations/{invitation_id}/resend",
            post(clubs::resend_invitation),
        )
        .route(
            "/clubs/{club_slug}/invitations/{invitation_id}",
            delete(clubs::revoke_invitation),
        )
        .route("/clubs/{club_slug}/impersonate", post(clubs::impersonate))
        .route("/clubs/{club_slug}/audit-log", get(clubs::audit_log));

    let cors = build_cors(&state);
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1", api)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Liveness/readiness for the reverse proxy: checks the database connection.
async fn healthz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<&'static str, crate::errors::AppError> {
    state.db.ping().await?;
    Ok("ok")
}

/// Strict CORS: no cross-origin access unless origins are explicitly
/// configured (Fosse shipped `permissive()` in production — never again).
fn build_cors(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();
    if origins.is_empty() {
        return CorsLayer::new();
    }
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
}
