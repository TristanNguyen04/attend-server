//! Database objects used by more than one module. Applied before any module's tables.
use sea_orm_migration::prelude::*;

mod m20261018_000000_create_shared_objects;

#[must_use]
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20261018_000000_create_shared_objects::Migration)]
}
