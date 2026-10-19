//! Migrations of the `identity` module. Add new ones here, oldest first.
use sea_orm_migration::prelude::*;

mod m20261018_000001_create_schema_identity;
mod m20261018_000002_create_identity_tables;
mod m20261019_000001_require_canonical_email_identity;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20261018_000001_create_schema_identity::Migration),
        Box::new(m20261018_000002_create_identity_tables::Migration),
        Box::new(m20261019_000001_require_canonical_email_identity::Migration),
    ]
}
