//! Membership read services exposed to other modules (module-boundary rule:
//! no direct entity access across modules).

use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::tenancy::entities::{club_roles, memberships, prelude::*};

/// Active membership of `user_id` in `club_id`, if any. THE tenant check for
/// user-global resources accessed through a club route.
pub async fn find_active(
    db: &DatabaseConnection,
    club_id: Uuid,
    user_id: Uuid,
) -> Result<Option<memberships::Model>, AppError> {
    Ok(Memberships::find()
        .filter(memberships::Column::ClubId.eq(club_id))
        .filter(memberships::Column::UserId.eq(user_id))
        .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
        .one(db)
        .await?)
}

/// All ACTIVE member user ids of a club — bounded by club size (a few
/// hundred rows); callers filter/join on user-global data afterwards.
pub async fn active_user_ids(
    db: &DatabaseConnection,
    club_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    Ok(Memberships::find()
        .filter(memberships::Column::ClubId.eq(club_id))
        .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
        .all(db)
        .await?
        .into_iter()
        .map(|m| m.user_id)
        .collect())
}

pub struct MembershipPage {
    pub rows: Vec<(memberships::Model, Option<users::Model>)>,
    pub roles: Vec<club_roles::Model>,
    pub total: u64,
}

/// One page of a club's memberships with their users and the club roles
/// (two queries + count — callers must not add per-row queries).
pub async fn page_with_users(
    db: &DatabaseConnection,
    club_id: Uuid,
    page: u64,
    per_page: u64,
) -> Result<MembershipPage, AppError> {
    let paginator = Memberships::find()
        .filter(memberships::Column::ClubId.eq(club_id))
        .find_also_related(crate::identity::entities::prelude::Users)
        .order_by_asc(memberships::Column::CreatedAt)
        .paginate(db, per_page);
    let total = paginator.num_items().await? as u64;
    let rows = paginator.fetch_page(page).await?;
    let roles = ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(club_id))
        .all(db)
        .await?;
    Ok(MembershipPage { rows, roles, total })
}

/// Whether `user_id` has any membership (whatever its status) in `club_id`.
pub async fn exists<C: ConnectionTrait>(
    conn: &C,
    club_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    Ok(Memberships::find()
        .filter(memberships::Column::ClubId.eq(club_id))
        .filter(memberships::Column::UserId.eq(user_id))
        .count(conn)
        .await?
        > 0)
}

/// Creates an active membership. The caller has checked that `role_id`
/// belongs to `club_id` (tenant isolation) and that no membership exists
/// (the UNIQUE (club_id, user_id) constraint is the backstop → 409).
pub async fn create<C: ConnectionTrait>(
    conn: &C,
    club_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<memberships::Model, AppError> {
    let created = crate::now();
    Ok(memberships::ActiveModel {
        id: Set(Uuid::now_v7()),
        club_id: Set(club_id),
        user_id: Set(user_id),
        role_id: Set(role_id),
        status: Set(memberships::STATUS_ACTIVE.to_string()),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

/// Number of memberships (any status, any club) of `user_id` — feeds the
/// "club-managed account" predicate (auth-onboarding design R4).
pub async fn count_for_user<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<u64, AppError> {
    Ok(Memberships::find()
        .filter(memberships::Column::UserId.eq(user_id))
        .count(conn)
        .await?)
}
