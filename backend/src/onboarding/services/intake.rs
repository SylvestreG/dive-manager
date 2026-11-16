//! Member intake: one person entering a club on an admin's behalf. This is
//! the single-row engine shared by manual creation (slice 1) and, later, CSV
//! import rows and access-request approvals.
//!
//! Rules (auth-onboarding design §3):
//! - an email that matches an existing account adds THAT account to the
//!   club (users are global; no duplicate person);
//! - an existing account is never overwritten; empty profile fields are
//!   filled only on accounts the club manages (R4), differing ones are
//!   reported (R22); a name mismatch needs explicit confirmation (R23);
//! - no email ⇒ a new account without one (reachable later through SSO
//!   linking or by setting an email).

use sea_orm::*;
use serde::Serialize;
use serde_json::json;

use crate::errors::AppError;
use crate::ffessm_licence::parse::fold;
use crate::identity::entities::users;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::{emails, users as user_service};
use crate::tenancy::entities::{club_roles, clubs};
use crate::tenancy::services::memberships;
use crate::users::services::profiles::{self, FieldDiff, FillMode, IntakeFields};
use crate::AppState;

#[derive(Debug, Clone)]
pub struct NewMember {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub profile: IntakeFields,
    /// The admin confirmed that a "check" match (email known, names differ)
    /// is the same person.
    pub confirm_existing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntakeOutcome {
    /// A new account was created.
    Created,
    /// An existing account (matched by email) joined the club.
    AddedExisting,
    /// The email matches an account whose names differ: NOTHING was written;
    /// the admin must confirm (`confirm_existing`) or fix the email (R23).
    Check,
}

#[derive(Debug)]
pub struct IntakeResult {
    pub user: users::Model,
    /// `None` for `Check` (nothing written).
    pub membership_id: Option<uuid::Uuid>,
    pub outcome: IntakeOutcome,
    /// Differences with the stored account — shown for review, never applied.
    pub diffs: Vec<FieldDiff>,
}

/// Accent/case/punctuation-insensitive name equality ("Éric" = "ERIC").
fn same_name(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

/// "Club-managed" for an account that is NOT yet a member of the acting club
/// (design R4): never logged in, not a global admin, member of no club.
/// Only such accounts may have their profile filled by a club's intake.
async fn is_club_managed_newcomer<C: ConnectionTrait>(
    conn: &C,
    user: &users::Model,
) -> Result<bool, AppError> {
    Ok(user.activated_at.is_none()
        && !user.is_global_admin
        && memberships::count_for_user(conn, user.id).await? == 0)
}

/// Adds one person to `club` with `role`, inside the caller's transaction.
/// Authorization (permission, admin-role gating) is the caller's job.
///
/// Email match on an existing account:
/// - names differ and not confirmed → `Check`, nothing written;
/// - otherwise the account joins the club; its profile is filled only when
///   the club manages it (R4) — never for an account that logged in, belongs
///   to another club, or is a global admin (report-only diffs).
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

    let Some(user) = existing else {
        let user = user_service::create_user(conn, email, &new.first_name, &new.last_name).await?;
        let membership = memberships::create(conn, club.id, user.id, role.id).await?;
        let merge = profiles::merge_intake(conn, user.id, new.profile, FillMode::Fill).await?;
        audit::record(
            conn,
            actor,
            Some(club.id),
            "member.created",
            "membership",
            Some(membership.id),
            json!({ "user_id": user.id, "role_id": role.id, "filled": merge.filled }),
        )
        .await?;
        return Ok(IntakeResult {
            user,
            membership_id: Some(membership.id),
            outcome: IntakeOutcome::Created,
            diffs: merge.diffs,
        });
    };

    if user.status == users::STATUS_DISABLED {
        return Err(AppError::Conflict("this account is disabled".to_string()));
    }
    if memberships::exists(conn, club.id, user.id).await? {
        return Err(AppError::Conflict(
            "this person is already a member of the club".to_string(),
        ));
    }

    let names_match =
        same_name(&new.first_name, &user.first_name) && same_name(&new.last_name, &user.last_name);
    let name_diff = (!names_match).then(|| FieldDiff {
        field: "name",
        stored: Some(format!("{} {}", user.first_name, user.last_name)),
        incoming: Some(format!(
            "{} {}",
            new.first_name.trim(),
            new.last_name.trim()
        )),
    });

    // Serialize concurrent intakes of the same account (two clubs adding the
    // same newcomer must not both see it as club-managed).
    let user = user_service::lock_for_update(conn, user.id).await?;
    let managed = is_club_managed_newcomer(conn, &user).await?;
    if !names_match && !new.confirm_existing {
        // The person is NOT a member of the acting club: reveal nothing of
        // their profile — only that the names differ (and which fields
        // would differ, without values). Audited: it discloses the holder's
        // name to the acting club.
        let merge =
            profiles::merge_intake(conn, user.id, new.profile, FillMode::ReportOnly).await?;
        let mut diffs: Vec<FieldDiff> = merge.diffs.into_iter().map(FieldDiff::redacted).collect();
        diffs.splice(0..0, name_diff);
        audit::record(
            conn,
            actor,
            Some(club.id),
            "member.intake_check",
            "user",
            Some(user.id),
            json!({ "diff_fields": diffs.iter().map(|d| d.field).collect::<Vec<_>>() }),
        )
        .await?;
        return Ok(IntakeResult {
            user,
            membership_id: None,
            outcome: IntakeOutcome::Check,
            diffs,
        });
    }

    let membership = memberships::create(conn, club.id, user.id, role.id).await?;
    let mode = if managed {
        FillMode::Fill
    } else {
        FillMode::ReportOnly
    };
    let merge = profiles::merge_intake(conn, user.id, new.profile, mode).await?;
    // Stored values of an account shared with other clubs (or that logged
    // in) are its owner's data: field names only. The member sheet shows
    // the profile through the regular member-view rules.
    let mut diffs: Vec<FieldDiff> = if managed {
        merge.diffs
    } else {
        merge.diffs.into_iter().map(FieldDiff::redacted).collect()
    };
    diffs.splice(0..0, name_diff);

    audit::record(
        conn,
        actor,
        Some(club.id),
        "member.added",
        "membership",
        Some(membership.id),
        json!({
            "user_id": user.id,
            "role_id": role.id,
            "confirmed_name_mismatch": !names_match,
            "filled": merge.filled,
            "diff_fields": diffs.iter().map(|d| d.field).collect::<Vec<_>>(),
        }),
    )
    .await?;

    Ok(IntakeResult {
        user,
        membership_id: Some(membership.id),
        outcome: IntakeOutcome::AddedExisting,
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
