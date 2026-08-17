use chrono::Duration;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::services::users as user_service;
use crate::identity::services::{audit, audit::Actor, login, sessions, tokens};
use crate::tenancy::entities::{club_roles, clubs, invitations, memberships, prelude::*};
use crate::tenancy::services::emails;
use crate::{normalize_email, now, validate_email, AppState};

pub struct NewInvitation<'a> {
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    /// None = global-administrator invitation.
    pub club: Option<&'a clubs::Model>,
    /// Required for club invitations; must belong to the club.
    pub role: Option<&'a club_roles::Model>,
}

/// Creates (or replaces) a pending invitation and emails its link.
pub async fn create_invitation(
    state: &AppState,
    actor: Actor,
    inviter_name: &str,
    new: NewInvitation<'_>,
) -> Result<invitations::Model, AppError> {
    let email = normalize_email(new.email);
    validate_email(&email)?;
    if new.first_name.trim().is_empty() || new.last_name.trim().is_empty() {
        return Err(AppError::Validation(
            "first name and last name are required".to_string(),
        ));
    }

    let (club_id, role_id) = match (new.club, new.role) {
        (Some(club), Some(role)) => {
            if role.club_id != club.id {
                // Tenant isolation: a role from another club is not visible.
                return Err(AppError::NotFound("role not found".to_string()));
            }
            (Some(club.id), Some(role.id))
        }
        (Some(_), None) => {
            return Err(AppError::Validation(
                "a role is required for a club invitation".to_string(),
            ))
        }
        (None, _) => (None, None),
    };

    // An existing account is fine (multi-club users) — but not one that
    // already holds what the invitation grants.
    if let Some(user) = user_service::find_by_email(&state.db, &email).await? {
        if user.status == users::STATUS_DISABLED {
            return Err(AppError::Conflict("this account is disabled".to_string()));
        }
        match club_id {
            Some(club_id) => {
                let already_member = Memberships::find()
                    .filter(memberships::Column::ClubId.eq(club_id))
                    .filter(memberships::Column::UserId.eq(user.id))
                    .one(&state.db)
                    .await?
                    .is_some();
                if already_member {
                    return Err(AppError::Conflict(
                        "this user is already a member of the club".to_string(),
                    ));
                }
            }
            None => {
                if user.is_global_admin {
                    return Err(AppError::Conflict(
                        "this user is already a global administrator".to_string(),
                    ));
                }
            }
        }
    }

    let token = tokens::generate();
    let created = now();
    let txn = state.db.begin().await?;

    // At most one pending invitation per (email, scope).
    let mut revoke = Invitations::update_many()
        .col_expr(invitations::Column::RevokedAt, Expr::value(Some(created)))
        .filter(invitations::Column::Email.eq(email.clone()))
        .filter(invitations::Column::ConsumedAt.is_null())
        .filter(invitations::Column::RevokedAt.is_null());
    revoke = match club_id {
        Some(club_id) => revoke.filter(invitations::Column::ClubId.eq(club_id)),
        None => revoke.filter(invitations::Column::ClubId.is_null()),
    };
    revoke.exec(&txn).await?;

    let invitation = invitations::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(email.clone()),
        first_name: Set(new.first_name.trim().to_string()),
        last_name: Set(new.last_name.trim().to_string()),
        club_id: Set(club_id),
        role_id: Set(role_id),
        invited_by: Set(actor.user_id),
        token_hash: Set(token.hash),
        expires_at: Set(created + Duration::days(state.config.auth.invitation_days)),
        consumed_at: Set(None),
        revoked_at: Set(None),
        created_at: Set(created),
    }
    .insert(&txn)
    .await?;

    audit::record(
        &txn,
        actor,
        club_id,
        "invitation.created",
        "invitation",
        Some(invitation.id),
        json!({ "email": email, "role_id": role_id }),
    )
    .await?;
    txn.commit().await?;

    let url = format!("{}/auth/invitations/{}", state.config.base_url, token.token);
    let (subject, body) = emails::invitation_email(
        &invitation.first_name,
        inviter_name,
        new.club.map(|c| c.name.as_str()),
        &url,
        state.config.auth.invitation_days,
    );
    state.mailer.send(&invitation.email, &subject, body).await?;
    Ok(invitation)
}

pub struct InvitationPreview {
    pub invitation: invitations::Model,
    pub club_name: Option<String>,
    pub role_name: Option<String>,
}

pub async fn find_valid_by_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<invitations::Model, AppError> {
    Invitations::find()
        .filter(invitations::Column::TokenHash.eq(tokens::hash(token)))
        .filter(invitations::Column::ConsumedAt.is_null())
        .filter(invitations::Column::RevokedAt.is_null())
        .filter(invitations::Column::ExpiresAt.gt(now()))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("invitation not found or expired".to_string()))
}

pub async fn preview_invitation(
    db: &DatabaseConnection,
    token: &str,
) -> Result<InvitationPreview, AppError> {
    let invitation = find_valid_by_token(db, token).await?;
    let club_name = match invitation.club_id {
        Some(club_id) => Clubs::find_by_id(club_id).one(db).await?.map(|c| c.name),
        None => None,
    };
    let role_name = match invitation.role_id {
        Some(role_id) => ClubRoles::find_by_id(role_id)
            .one(db)
            .await?
            .map(|r| r.name),
        None => None,
    };
    Ok(InvitationPreview {
        invitation,
        club_name,
        role_name,
    })
}

/// Accepts an invitation: finds or creates the user, grants the membership or
/// the global-admin flag, optionally sets a password, opens a session.
pub async fn accept_invitation(
    state: &AppState,
    token: &str,
    password: Option<&str>,
) -> Result<(String, users::Model), AppError> {
    let invitation = find_valid_by_token(&state.db, token).await?;
    if let Some(password) = password {
        // Validate before mutating anything.
        if password.len() < 8 || password.len() > 128 {
            return Err(AppError::Validation(
                "password must be between 8 and 128 characters".to_string(),
            ));
        }
    }

    let txn = state.db.begin().await?;

    let consumed = Invitations::update_many()
        .col_expr(invitations::Column::ConsumedAt, Expr::value(Some(now())))
        .filter(invitations::Column::Id.eq(invitation.id))
        .filter(invitations::Column::ConsumedAt.is_null())
        .exec(&txn)
        .await?;
    if consumed.rows_affected != 1 {
        return Err(AppError::NotFound(
            "invitation not found or expired".to_string(),
        ));
    }

    let existing = user_service::find_by_email(&txn, &invitation.email).await?;
    let mut user = match existing {
        Some(user) if user.status == users::STATUS_DISABLED => {
            return Err(AppError::Forbidden("this account is disabled".to_string()));
        }
        Some(user) => user_service::activate_if_invited(&txn, user).await?,
        None => {
            user_service::create_active_user(
                &txn,
                &invitation.email,
                &invitation.first_name,
                &invitation.last_name,
            )
            .await?
        }
    };

    match (invitation.club_id, invitation.role_id) {
        (Some(club_id), Some(role_id)) => {
            let already_member = Memberships::find()
                .filter(memberships::Column::ClubId.eq(club_id))
                .filter(memberships::Column::UserId.eq(user.id))
                .one(&txn)
                .await?
                .is_some();
            if already_member {
                return Err(AppError::Conflict(
                    "already a member of this club".to_string(),
                ));
            }
            let created = now();
            memberships::ActiveModel {
                id: Set(Uuid::now_v7()),
                club_id: Set(club_id),
                user_id: Set(user.id),
                role_id: Set(role_id),
                status: Set(memberships::STATUS_ACTIVE.to_string()),
                created_at: Set(created),
                updated_at: Set(created),
            }
            .insert(&txn)
            .await?;
        }
        (None, _) => {
            user = user_service::set_global_admin(&txn, user, true).await?;
        }
        (Some(_), None) => {
            tracing::error!(invitation_id = %invitation.id, "club invitation without role");
            return Err(AppError::Internal);
        }
    }

    if let Some(password) = password {
        login::set_password(&txn, user.id, password).await?;
    }

    audit::record(
        &txn,
        Actor {
            user_id: user.id,
            impersonated_user_id: None,
        },
        invitation.club_id,
        "invitation.accepted",
        "invitation",
        Some(invitation.id),
        json!({ "invited_by": invitation.invited_by }),
    )
    .await?;

    let (session_token, _) =
        sessions::create_session(&txn, &state.config, user.id, None, None).await?;
    txn.commit().await?;
    Ok((session_token, user))
}

/// Revokes a pending invitation.
pub async fn revoke_invitation<C: ConnectionTrait>(
    conn: &C,
    actor: Actor,
    invitation: &invitations::Model,
) -> Result<(), AppError> {
    if invitation.consumed_at.is_some() || invitation.revoked_at.is_some() {
        return Err(AppError::Conflict(
            "invitation is no longer pending".to_string(),
        ));
    }
    Invitations::update_many()
        .col_expr(invitations::Column::RevokedAt, Expr::value(Some(now())))
        .filter(invitations::Column::Id.eq(invitation.id))
        .filter(invitations::Column::RevokedAt.is_null())
        .exec(conn)
        .await?;
    audit::record(
        conn,
        actor,
        invitation.club_id,
        "invitation.revoked",
        "invitation",
        Some(invitation.id),
        json!({ "email": invitation.email }),
    )
    .await?;
    Ok(())
}
