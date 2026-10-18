#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
//! One migrator for the whole app. Each module owns a folder of migrations and its own
//! Postgres schema; this file only concatenates them, lowest layer first.
pub use sea_orm_migration::prelude::*;

mod calendar;
mod capture;
mod courses;
mod identity;
mod records;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations = Vec::new();
        migrations.extend(calendar::migrations());
        migrations.extend(identity::migrations());
        migrations.extend(courses::migrations());
        migrations.extend(capture::migrations());
        migrations.extend(records::migrations());
        migrations
    }
}
