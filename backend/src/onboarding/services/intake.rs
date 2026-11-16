//! Member intake: one person entering a club on an admin's behalf. This is
//! the single-row engine shared by manual creation (slice 1) and, later, CSV
//! import rows and access-request approvals.
//!
//! Rules (auth-onboarding design §3):
//! - an email that matches an existing account adds THAT account to the
//!   club (users are global; no duplicate person);
//! - an existing account is never overwritten: empty profile fields are
//!   filled, differing ones are reported (R22);
//! - no email ⇒ a new account without one (reachable later through SSO
//!   linking or by setting an email).

use sea_orm::*;
use serde::Serialize;
use serde_json::json;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::{emails, users as user_service};
use crate::tenancy::entities::{club_roles, clubs};
use crate::tenancy::services::memberships;
use crate::users::services::profiles::{self, FieldDiff, IntakeFields};
use crate::AppState;

#[derive(Debug, Clone)]
pub struct NewMember {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub profile: IntakeFields,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntakeOutcome {
    /// A new account was created.
    Created,
    /// An existing account (matched by email) joined the club.
    AddedExisting,
}

#[derive(Debug)]
pub struct IntakeResult {
    pub user: users::Model,
    pub membership_id: uuid::Uuid,
    pub outcome: IntakeOutcome,
    /// Fields of an existing account that differ from what the admin
    /// entered — shown for review, never applied.
    pub diffs: Vec<FieldDiff>,
}

/// Adds one person to `club` with `role`, inside the caller's transaction.
/// Authorization (permission, admin-role gating) is the caller's job.
pub async fn add_member<C: ConnectionTrait>(
    conn: &C,
    actor: Actor,
    club: &clubs::Model,
    role: &club_roles::Model,
    new: NewMember,
) -> Result<IntakeResult, AppError> {
    if role.club_id != club.id {
        // Tenant isolation: another club's role is not visible here.
        return Err(AppError::NotFound("role not found".to_string()));
    }
    let email = new
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty());

    let existing = match email {
        Some(email) => user_service::find_by_email(conn, email).await?,
        None => None,
    };

    let (user, outcome) = match existing {
        Some(user) => {
            if user.status == users::STATUS_DISABLED {
                return Err(AppError::Conflict("this account is disabled".to_string()));
            }
            if memberships::exists(conn, club.id, user.id).await? {
                return Err(AppError::Conflict(
                    "this person is already a member of the club".to_string(),
                ));
            }
            (user, IntakeOutcome::AddedExisting)
        }
        None => (
            user_service::create_user(conn, email, &new.first_name, &new.last_name).await?,
            IntakeOutcome::Created,
        ),
    };

    let membership = memberships::create(conn, club.id, user.id, role.id).await?;
    let mut diffs = profiles::fill_gaps(conn, user.id, new.profile).await?;
    if outcome == IntakeOutcome::AddedExisting {
        let entered = (new.first_name.trim(), new.last_name.trim());
        if !entered.0.eq_ignore_ascii_case(&user.first_name)
            || !entered.1.eq_ignore_ascii_case(&user.last_name)
        {
            diffs.insert(
                0,
                FieldDiff {
                    field: "name",
                    stored: format!("{} {}", user.first_name, user.last_name),
                    incoming: format!("{} {}", entered.0, entered.1),
                },
            );
        }
    }

    audit::record(
        conn,
        actor,
        Some(club.id),
        match outcome {
            IntakeOutcome::Created => "member.created",
            IntakeOutcome::AddedExisting => "member.added",
        },
        "membership",
        Some(membership.id),
        json!({ "user_id": user.id, "role_id": role.id, "diffs": diffs }),
    )
    .await?;

    Ok(IntakeResult {
        user,
        membership_id: membership.id,
        outcome,
        diffs,
    })
}

/// Best-effort welcome email (no token inside — ADR-0009). Call AFTER the
/// transaction committed; a send failure is logged, never surfaced: the
/// account exists either way and the person can request a link themselves.
pub async fn send_welcome(state: &AppState, user: &users::Model, club_name: Option<&str>) {
    let Some(email) = user.email.as_deref() else {
        return;
    };
    let url = emails::login_url(&state.config.base_url, email);
    let (subject, body) = emails::welcome_email(&user.first_name, club_name, &url);
    if let Err(error) = state.mailer.send(email, &subject, body).await {
        tracing::error!(?error, user_id = %user.id, "failed to send welcome email");
    }
}
