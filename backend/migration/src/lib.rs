pub use sea_orm_migration::prelude::*;

mod m20260817_000001_foundation;
mod m20260817_000002_user_profiles;
mod m20260819_000003_competencies;
mod m20261116_000004_auth_simplify;
mod m20261117_000005_oidc;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260817_000001_foundation::Migration),
            Box::new(m20260817_000002_user_profiles::Migration),
            Box::new(m20260819_000003_competencies::Migration),
            Box::new(m20261116_000004_auth_simplify::Migration),
            Box::new(m20261117_000005_oidc::Migration),
        ]
    }
}
