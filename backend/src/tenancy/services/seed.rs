use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::tenancy::entities::{club_modules, club_roles};

/// Known club-toggleable modules. Business modules register here as they are
/// built; toggles for not-yet-implemented modules simply have no effect.
pub const MODULES: [&str; 7] = [
    "members",
    "activities",
    "registrations",
    "palanquees",
    "competencies",
    "imports",
    "notifications",
];

pub const ROLE_ADMIN: &str = "Administrateur";
pub const ROLE_MEMBER: &str = "Membre";

/// Seeds a new club: the two default roles (ADR-0007 — instructor is a
/// derived ffessm capability, not a role; Membre carries no permission by
/// default — fail-closed, clubs may grant some via the Roles page) and one
/// disabled row per known module. Returns the admin role id (used to invite
/// the club's first administrator).
pub async fn seed_club<C: ConnectionTrait>(conn: &C, club_id: Uuid) -> Result<Uuid, AppError> {
    let admin_role_id = Uuid::now_v7();

    let roles = [
        (admin_role_id, ROLE_ADMIN, true),
        (Uuid::now_v7(), ROLE_MEMBER, false),
    ];
    club_roles::Entity::insert_many(roles.into_iter().map(|(id, name, is_admin)| {
        club_roles::ActiveModel {
            id: Set(id),
            club_id: Set(club_id),
            name: Set(name.to_string()),
            is_admin: Set(is_admin),
        }
    }))
    .exec(conn)
    .await?;

    club_modules::Entity::insert_many(MODULES.into_iter().map(|module| {
        club_modules::ActiveModel {
            club_id: Set(club_id),
            module: Set(String::from(module)),
            enabled: Set(false),
        }
    }))
    .exec(conn)
    .await?;

    Ok(admin_role_id)
}
