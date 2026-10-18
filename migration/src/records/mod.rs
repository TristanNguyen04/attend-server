//! Migrations of the `records` module. Add new ones here, oldest first.
use sea_orm_migration::prelude::*;

mod m20261018_000001_create_schema_records;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20261018_000001_create_schema_records::Migration)]
}
