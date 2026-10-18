# Tasks: Persist the approved data model

Derived from `requirements.md`. There is no Gherkin scenario (no user-visible behaviour), so
each task is **TDD red -> TDD green**: write the schema tests first and see them fail, then add
the migration that makes them pass.

Every module's migration lives in `migration/src/<module>/` as a Rust file plus one `.sql` file.
Migrations are forward-only: there is no `down`.

---

## T0: Shared database objects  ·  FR4
- **TDD red**: tests that every table with `updated_at` has the `set_updated_at` trigger and
  that an update moves `updated_at`.
- **TDD green**: migration creating `public.set_updated_at()`, placed first in the migrator.

## T1: `calendar`  ·  FR1, FR2, FR3, FR7
- **TDD red**: `terms` exists in `calendar` with a UUID key, timestamps, unique `name`.
- **TDD green**: `calendar` tables migration (`terms`).

## T2: `identity`  ·  FR1-FR3, FR5-FR7, FR11
- **TDD red**: tests for `users`, `devices`, `device_lifecycle_items`: value sets, one active
  device per user and term, unique email and public key, status must change.
- **TDD green**: `identity` tables migration.

## T3: `courses`  ·  FR1-FR3, FR5
- **TDD red**: `courses` and `course_members` (composite key, value sets).
- **TDD green**: `courses` tables migration.

## T4: `capture`  ·  FR1-FR3, FR5-FR9
- **TDD red**: session time check, room overlap (cancelled and adjacent cases), unique tag UID,
  scan event uniqueness rules.
- **TDD green**: `capture` tables migration, enabling `btree_gist`.

## T5: `records`  ·  FR1-FR3, FR5, FR6, FR10, FR11
- **TDD red**: one record per session and student, one pending appeal per record (re-appeal
  allowed), lifecycle status change and single-cause checks.
- **TDD green**: `records` tables migration.

## T6: Cross-cutting checks  ·  FR1, FR3, FR6, FR12
- **TDD red**: exact table list per schema, exact foreign key list (none across modules), exact
  timestamp columns, migrations are forward-only (a revert fails and changes nothing; no
  `.down.sql` files exist).
- **TDD green**: nothing new if T0-T5 are correct; otherwise fix the offending migration.

---

## Cross-cutting notes
- Write all the schema tests first; they fail until each migration exists.
- Entities are generated from the migrated database afterwards (`docs/architecture.md`,
  "Adding a table to a module"); they are not part of this feature's tests.
