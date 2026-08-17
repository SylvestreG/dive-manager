pub mod auth;
pub mod me;

use serde::Serialize;

use crate::identity::entities::users;

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: uuid::Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_global_admin: bool,
}

impl From<&users::Model> for UserDto {
    fn from(user: &users::Model) -> Self {
        UserDto {
            id: user.id,
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            is_global_admin: user.is_global_admin,
        }
    }
}

/// Response for every endpoint that opens a session.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub token: String,
    pub user: UserDto,
}
