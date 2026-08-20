//! Self-service competency views. Reads only — validations are written by
//! instructors through the club routes. Reads work while impersonating (the
//! admin sees what the member sees).

use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;

use crate::competencies::handlers::{competency_view, documents, CompetencyViewDto};
use crate::competencies::services::tree;
use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::users::services::profiles;
use crate::AppState;

#[derive(Deserialize)]
pub struct MyLevelQuery {
    pub certification: Option<String>,
}

/// Defaults to the certification the user prepares; 422 when neither the
/// parameter nor a preparation exists (the UI shows its empty state).
async fn resolve_level(
    state: &AppState,
    user_id: uuid::Uuid,
    query: &MyLevelQuery,
) -> Result<ffessm::Certification, AppError> {
    if let Some(code) = &query.certification {
        return tree::parse_certification(code);
    }
    let preparing = profiles::get(&state.db, user_id)
        .await?
        .and_then(|p| p.preparing_certification);
    match preparing {
        Some(code) => tree::parse_certification(&code),
        None => Err(AppError::Validation(
            "no certification in preparation".to_string(),
        )),
    }
}

pub async fn get_my_competencies(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Query(query): Query<MyLevelQuery>,
) -> Result<Json<CompetencyViewDto>, AppError> {
    let certification = resolve_level(&state, auth.effective_user.id, &query).await?;
    Ok(Json(
        competency_view(&state.db, auth.effective_user.id, certification).await?,
    ))
}

pub async fn get_my_document(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Query(query): Query<MyLevelQuery>,
) -> Result<Response, AppError> {
    let certification = resolve_level(&state, auth.effective_user.id, &query).await?;
    documents::generate_for_user(&state, &auth.effective_user, certification).await
}
