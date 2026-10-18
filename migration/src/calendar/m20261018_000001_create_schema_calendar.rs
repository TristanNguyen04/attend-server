use sea_orm_migration::prelude::*;

/// Creates the Postgres schema owned by the `calendar` module.
///
/// Forward-only: there is no `down`. A mistake is fixed by a new migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("CREATE SCHEMA IF NOT EXISTS calendar")
            .await?;
        Ok(())
    }
}
