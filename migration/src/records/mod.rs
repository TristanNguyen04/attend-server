//! Migrations of the `records` module. Add new ones here, oldest first.
use sea_orm_migration::prelude::*;

mod m20261018_000001_create_schema_records;
mod m20261018_000002_create_records_tables;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20261018_000001_create_schema_records::Migration),
        Box::new(m20261018_000002_create_records_tables::Migration),
    ]
}
