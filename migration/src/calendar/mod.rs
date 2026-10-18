//! Migrations of the `calendar` module. Add new ones here, oldest first.
use sea_orm_migration::prelude::*;

mod m20261018_000001_create_schema_calendar;
mod m20261018_000002_create_calendar_tables;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20261018_000001_create_schema_calendar::Migration),
        Box::new(m20261018_000002_create_calendar_tables::Migration),
    ]
}
