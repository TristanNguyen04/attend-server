# Feature: Persist the approved data model

Module: cross-module (`platform`)
Stories: none (enabling work for every module)

## Goal

> **As a** developer,
> **I want** every table of the approved ER diagram stored in its module's schema with the
> agreed keys, value sets and constraints,
> **so that** features can be built on a data model the database itself protects.

This work has no user-visible behaviour, so it has no Cucumber scenario. Its tests are
schema tests in `tests/schema.rs` (the TDD layer).

## Definitions

- **Module schema**: the Postgres schema named after the module that owns the table
  (`calendar`, `identity`, `courses`, `capture`, `records`).
- **Reference by ID**: a column that holds the key of a row owned by another module, without a
  foreign key (`docs/architecture.md`, rule 6).
- **Value set**: the list of allowed values of a status-like column, written in
  `docs/er-diagram.md`.
- **Lifecycle table**: `device_lifecycle_items`, `attendance_record_lifecycle_items` and
  `appeal_lifecycle_items`.

## Functional requirements (EARS)

**FR1: Schema ownership (ubiquitous).**
The system shall store each of the 14 tables in the schema of the module that owns it, and in
no other schema.

**FR2: UUID keys (ubiquitous).**
The system shall give every table a `uuid` primary key `id` with the database default
`gen_random_uuid()`, except `course_members`, whose primary key is `(course_id, user_id)`.

**FR3: Timestamps (ubiquitous).**
The system shall give every table `created_at` and `updated_at` columns of type `timestamptz`,
not null, defaulting to `now()`, except the three lifecycle tables, which are append-only and have only `created_at`.
No column shall use a timestamp type without a time zone.

**FR4: Updated-at maintenance (event-driven).**
When a row of a table with `updated_at` is updated, the database shall set `updated_at` to the
current time.

**FR5: Value sets (unwanted behaviour).**
If a value outside the agreed value set is written to `users.status`, `course_members.role`,
`course_members.status`, `nfc_tags.status`, `devices.platform`, `devices.status`,
`scan_events.source`, `scan_events.result`, `attendance_records.status`, `appeals.status`, or to
`old_status` / `new_status` of a lifecycle table, then the database shall reject the write.

**FR6: No foreign keys across modules (ubiquitous).**
The system shall not define a foreign key whose referencing table and referenced table belong to
different modules. It shall define foreign keys for references inside a module.

**FR7: Devices and tags (unwanted behaviour).**
If a second ACTIVE device is written for the same `(user_id, term_id)`, then the database shall
reject it. `users.email`, `terms.name`, `devices.public_key` and `nfc_tags.tag_uid` shall be
unique, and so shall `(building, room_code)` in `rooms` and `(term_id, code)` in `courses`.

**FR8: Sessions (unwanted behaviour).**
If a session ends at or before it starts, or overlaps another non-cancelled session in the same
room, then the database shall reject it. A cancelled session shall not block its room, and a
session that starts exactly when another ends shall be accepted.

**FR9: Scan events (unwanted behaviour).**
If the same `(device_id, tag_id, tag_counter)` is written twice, or two ACCEPTED scans share
`(tag_id, tag_counter)`, then the database shall reject the write. REJECTED scans from different
devices may share a `(tag_id, tag_counter)`.

**FR10: Records and appeals (unwanted behaviour).**
If a second record is written for the same `(session_id, student_id)`, or a second PENDING appeal
for the same record, then the database shall reject it. A new appeal after a rejection shall be
accepted.

**FR11: Lifecycle rows (unwanted behaviour).**
If a lifecycle row has `old_status` equal to `new_status`, then the database shall reject it.
If an `attendance_record_lifecycle_items` row sets both `scan_event_id` and `appeal_id`, then the
database shall reject it.

**FR12: Forward-only migrations (unwanted behaviour).**
The system shall apply migrations only forward. If a revert is requested, then it shall fail
and leave the database unchanged. A mistake is fixed by a new migration, never by editing or
undoing one that has been merged.

## Non-functional requirements

**NFR1: Reviewable SQL.**
Each module's tables shall be created by one plain SQL file next to its migration, so a reviewer
reads the DDL directly.

**NFR2: Lookup indexes.**
Columns that other rows are looked up by (the foreign key and reference-by-ID columns listed in
`docs/er-diagram.md`) shall be indexed, directly or as the first column of a unique index.

## Decisions

| # | Question | Decision |
|---|---|---|
| Q1 | Postgres enums or `text` with `CHECK`? | `text` with `CHECK`. Adding a value later is a simple migration; enums are awkward to change. |
| Q2 | Who maintains `updated_at`? | A database trigger, so every writer is covered, not only application code. |
| Q3 | `reason` on lifecycle rows | Nullable on all three lifecycle tables: a reason is optional context. |
| Q4 | `courses.policies` when unset | Nullable. Null means "not configured yet"; the course feature spec decides the behaviour. |
| Q5 | `appeals.supporting_documents` | `text[]`, not null, default empty array. |
| Q6 | `changed_by` on `appeal_lifecycle_items` | Not null: every appeal change is made by a person. On `attendance_record_lifecycle_items` it is null for system changes. |
| Q7 | Overlap rule for sessions | A Postgres exclusion constraint over `(room_id, tstzrange(starts_at, ends_at))`, which needs the `btree_gist` extension. |
| Q8 | Unique `(building, room_code)` for rooms and `(term_id, code)` for courses | Both are unique, so two identical rooms cannot defeat the overlap rule and a term cannot list a course twice. |
| Q10 | Down migrations | None. Every migration is forward-only (decided by the team): there are no `down` steps or `.down.sql` files, and a bad change is corrected by a new migration. A local database is reset with `docker compose down -v`. |
| Q9 | Email case-insensitivity | **Decided in `specs/identity/sign-in` (Q5):** canonical form (trimmed, lower case) plus a `CHECK` constraint on `identity.users.email`. |
