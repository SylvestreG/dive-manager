use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::competencies::handlers::{
    documents as comp_documents, me as me_competencies, member as member_competencies,
    referential as comp_referential,
};
use crate::identity::handlers::{auth, me as me_password};
use crate::tenancy::handlers::{clubs, me, platform, public_invitations};
use crate::users::handlers::{
    catalog as users_catalog, me as me_profile, member as member_profile,
};
use crate::AppState;

/// Level-document templates arrive as base64 JSON (≤ 5 MiB raw): the default
/// 2 MiB body limit is raised on that route only.
const DOCUMENT_UPLOAD_BODY_LIMIT: usize = 8 * 1024 * 1024;

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        // Public authentication endpoints.
        .route("/auth/login-methods", post(auth::login_methods))
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
        .route("/me/identity", put(me_password::set_my_identity))
        // Self-service diving profile (user-global, no club in the URL).
        // Certifications are staff-entered only (product decision
        // 2026-08-18): no /me/certifications endpoints.
        .route(
            "/me/profile",
            get(me_profile::get_my_profile).put(me_profile::put_my_profile),
        )
        .route(
            "/me/stamp",
            get(me_profile::get_my_stamp)
                .put(me_profile::put_my_stamp)
                .delete(me_profile::delete_my_stamp),
        )
        // Each member imports their OWN official FFESSM licence (product
        // decision 2026-08-18) — fills the profile + brevets (`imported`).
        .route(
            "/me/ffessm/import",
            post(me_profile::import_my_ffessm_licence),
        )
        // Static FFESSM catalog (codes, labels, derived coverage) for the
        // certification checkbox tree — authenticated, nothing personal.
        .route("/catalog/certifications", get(users_catalog::list_catalog))
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
        // Reading the members list goes through /members (users module,
        // badges included); this route only writes memberships.
        .route(
            "/clubs/{club_slug}/memberships",
            post(clubs::add_membership),
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
        .route("/clubs/{club_slug}/audit-log", get(clubs::audit_log))
        // Member diving profiles (users module; target must be an active
        // member of the club — enforced inside the handlers).
        .route(
            "/clubs/{club_slug}/members",
            get(member_profile::list_members),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/profile",
            get(member_profile::get_member_profile).put(member_profile::put_member_profile),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/certifications",
            post(member_profile::record_certification),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/certifications/{certification_id}",
            delete(member_profile::delete_member_certification),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/training-status",
            put(member_profile::set_training_status),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/stamp",
            get(member_profile::get_member_stamp)
                .put(member_profile::put_member_stamp)
                .delete(member_profile::delete_member_stamp),
        )
        // Global competency referential (product decision 2026-08-19: the
        // tree is platform-global). Reads for every authenticated user;
        // writes for REAL global admins only (enforced in the handlers).
        .route("/competencies/levels", get(comp_referential::list_levels))
        .route("/competencies/tree", get(comp_referential::get_tree))
        .route(
            "/competencies/domains",
            post(comp_referential::create_domain),
        )
        .route(
            "/competencies/domains/{domain_id}",
            put(comp_referential::update_domain).delete(comp_referential::delete_domain),
        )
        .route(
            "/competencies/domains/{domain_id}/impact",
            get(comp_referential::domain_impact),
        )
        .route(
            "/competencies/modules",
            post(comp_referential::create_module),
        )
        .route(
            "/competencies/modules/{module_id}",
            put(comp_referential::update_module).delete(comp_referential::delete_module),
        )
        .route(
            "/competencies/modules/{module_id}/impact",
            get(comp_referential::module_impact),
        )
        .route("/competencies/skills", post(comp_referential::create_skill))
        .route(
            "/competencies/skills/{skill_id}",
            put(comp_referential::update_skill).delete(comp_referential::delete_skill),
        )
        .route(
            "/competencies/skills/{skill_id}/impact",
            get(comp_referential::skill_impact),
        )
        // Append-only validation history (real global admin).
        .route(
            "/competencies/validation-events",
            get(comp_referential::list_events),
        )
        // Level-document templates + stamp positions (real global admin).
        .route(
            "/competencies/levels/{certification}/document",
            get(comp_documents::download_document)
                .put(comp_documents::upload_document)
                .delete(comp_documents::delete_document)
                .layer(DefaultBodyLimit::max(DOCUMENT_UPLOAD_BODY_LIMIT)),
        )
        .route(
            "/competencies/levels/{certification}/positions",
            get(comp_documents::list_positions).put(comp_documents::batch_positions),
        )
        .route(
            "/competencies/levels/{certification}/positions/{skill_id}",
            put(comp_documents::upsert_position).delete(comp_documents::delete_position),
        )
        // Self-service competency view + filled PDF (user-global data).
        .route(
            "/me/competencies",
            get(me_competencies::get_my_competencies),
        )
        .route(
            "/me/competencies/document",
            get(me_competencies::get_my_document),
        )
        // Club-scoped competency routes: visibility via active membership,
        // validator gate = derived teaching grade (never a permission).
        .route(
            "/clubs/{club_slug}/competencies/students",
            get(member_competencies::list_students),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/competencies",
            get(member_competencies::get_member_competencies),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/competencies/document",
            get(member_competencies::get_member_document),
        )
        .route(
            "/clubs/{club_slug}/members/{user_id}/skill-validations/{skill_id}",
            put(member_competencies::put_skill_validation)
                .delete(member_competencies::delete_skill_validation),
        );

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
