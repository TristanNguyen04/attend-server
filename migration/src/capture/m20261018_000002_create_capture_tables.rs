use sea_orm_migration::prelude::*;

/// Creates the tables of the `capture` module.
///
/// Forward-only: there is no `down`. A mistake is fixed by a new migration, never by editing
/// this one once it is merged. The DDL is plain SQL in the `.sql` file next to this one.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("m20261018_000002_create_capture_tables.sql"))
            .await?;
        Ok(())
    }
}
