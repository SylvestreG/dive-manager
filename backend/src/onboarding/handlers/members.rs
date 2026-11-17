//! Club member intake endpoints (`/clubs/{slug}/members`, POST).

use axum::extract::State;
use axum::Json;
use chrono::NaiveDate;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::onboarding::services::intake::{self, IntakeOutcome, NewMember};
use crate::tenancy::context::Tenant;
use crate::tenancy::permissions::Permission;
use crate::tenancy::services::roles;
use crate::users::services::profiles::{FieldDiff, IntakeFields};
use crate::AppState;

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub phone: Option<String>,
    pub licence_number: Option<String>,
    pub role_id: Uuid,
    /// Welcome email to the account's address (no-op without one).
    #[serde(default = "default_true")]
    pub send_welcome: bool,
    /// Confirms a previous `check` outcome (email known, names differ).
    #[serde(default)]
    pub confirm_existing: bool,
}

/// The person as intake staff may see them — deliberately NOT `UserDto`
/// (no global-admin flag: intake must not become an account probe).
#[derive(Serialize)]
pub struct IntakeUserDto {
    pub id: Uuid,
    pub email: Option<String>,
    pub first_name: String,
    pub last_name: String,
}

impl From<&users::Model> for IntakeUserDto {
    fn from(user: &users::Model) -> Self {
        IntakeUserDto {
            id: user.id,
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct AddMemberResponse {
    pub user: IntakeUserDto,
    /// Absent for `check` (nothing was written).
    pub membership_id: Option<Uuid>,
    pub role_id: Uuid,
    pub role_name: String,
    pub is_admin: bool,
    pub outcome: IntakeOutcome,
    pub diffs: Vec<FieldDiff>,
}

/// Adds a person to the club: creates the account, or adds the existing
/// account matching the email (ADR-0009: admin-curated accounts, no
/// invitation). Member intake permission, real principal only; granting the
/// admin role additionally needs `club.roles.manage` (unchanged rule).
pub async fn add_member(
    State(state): State<AppState>,
    tenant: Tenant,
    Json(body): Json<AddMemberRequest>,
) -> Result<Json<AddMemberResponse>, AppError> {
    tenant.require_real(Permission::ClubIntakeManage)?;
    let role = roles::find_in_club(&state.db, tenant.club.id, body.role_id).await?;
    if role.is_admin {
        tenant.require_real(Permission::ClubRolesManage)?;
    }

    let txn = state.db.begin().await?;
    let result = intake::add_member(
        &txn,
        tenant.actor(),
        &tenant.club,
        &role,
        NewMember {
            first_name: body.first_name,
            last_name: body.last_name,
            email: body.email,
            profile: IntakeFields {
                birthdate: body.birthdate,
                phone: body.phone,
                licence_number: body.licence_number,
            },
            confirm_existing: body.confirm_existing,
        },
    )
    .await?;
    txn.commit().await?;

    if body.send_welcome && result.outcome != IntakeOutcome::Check {
        intake::send_welcome(&state, &result.user, Some(&tenant.club.name)).await;
    }

    // Stored profile values are member data: only callers who may view
    // members see them (the name diff stays — it is what "check" is about).
    let can_view_members = tenant.require(Permission::ClubMembersView).is_ok();
    let diffs = result
        .diffs
        .into_iter()
        .map(|d| {
            if can_view_members || d.field == "name" {
                d
            } else {
                d.redacted()
            }
        })
        .collect();

    Ok(Json(AddMemberResponse {
        user: IntakeUserDto::from(&result.user),
        membership_id: result.membership_id,
        role_id: role.id,
        role_name: role.name,
        is_admin: role.is_admin,
        outcome: result.outcome,
        diffs,
    }))
}
