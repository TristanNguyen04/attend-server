//! Migrations of the `courses` module. Add new ones here, oldest first.
use sea_orm_migration::prelude::*;

mod m20261018_000001_create_schema_courses;
mod m20261018_000002_create_courses_tables;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20261018_000001_create_schema_courses::Migration),
        Box::new(m20261018_000002_create_courses_tables::Migration),
    ]
}
