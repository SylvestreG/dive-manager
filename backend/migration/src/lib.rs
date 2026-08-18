pub use sea_orm_migration::prelude::*;

mod m20260817_000001_foundation;
mod m20260817_000002_user_profiles;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260817_000001_foundation::Migration),
            Box::new(m20260817_000002_user_profiles::Migration),
        ]
    }
}
