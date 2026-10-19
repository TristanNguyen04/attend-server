//! Schema tests: the approved ER (`docs/er-diagram.md`) as Postgres stores it.
//!
//! These are the TDD layer of `specs/platform/persistence`. Persistence has no user-visible
//! behaviour, so there is no Cucumber scenario. Every test boots the app, which migrates and
//! truncates the shared test database, so the tests must run one at a time (`#[serial]`).

use attend_server::app::App;
use loco_rs::testing::prelude::boot_test;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use serial_test::serial;

/// Every table, grouped by the module (and schema) that owns it.
const TABLES: &[(&str, &[&str])] = &[
    ("calendar", &["terms"]),
    ("identity", &["users", "devices", "device_lifecycle_items"]),
    ("courses", &["courses", "course_members"]),
    (
        "capture",
        &["rooms", "nfc_tags", "class_sessions", "scan_events"],
    ),
    (
        "records",
        &[
            "attendance_records",
            "attendance_record_lifecycle_items",
            "appeals",
            "appeal_lifecycle_items",
        ],
    ),
];

/// Append-only history tables that have `created_at` but no `updated_at`.
const CREATED_AT_ONLY: &[&str] = &[
    "identity.device_lifecycle_items",
    "records.attendance_record_lifecycle_items",
    "records.appeal_lifecycle_items",
];

const SCHEMAS: &str = "('calendar', 'identity', 'courses', 'capture', 'records')";
const CHECK: &str = "violates check constraint";
const UNIQUE: &str = "violates unique constraint";
const EXCLUSION: &str = "violates exclusion constraint";

// ------------------------------------------------------------------ helpers

async fn db() -> DatabaseConnection {
    boot_test::<App>()
        .await
        .expect("failed to boot the test app")
        .app_context
        .db
}

async fn run(db: &DatabaseConnection, sql: &str) -> Result<(), DbErr> {
    db.execute_unprepared(sql).await.map(|_| ())
}

/// The statement must succeed.
async fn ok(db: &DatabaseConnection, sql: &str) {
    if let Err(err) = run(db, sql).await {
        panic!("expected success, but got: {err}\nSQL: {sql}");
    }
}

/// The statement must fail with an error mentioning `expected`.
async fn rejected(db: &DatabaseConnection, sql: &str, expected: &str) {
    match run(db, sql).await {
        Ok(()) => panic!("expected `{expected}` but the statement succeeded\nSQL: {sql}"),
        Err(err) => assert!(
            err.to_string().contains(expected),
            "expected `{expected}` but got: {err}\nSQL: {sql}"
        ),
    }
}

/// The first column of every row, as text.
async fn strings(db: &DatabaseConnection, sql: &str) -> Vec<String> {
    db.query_all_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap_or_else(|err| panic!("query failed: {err}\nSQL: {sql}"))
        .iter()
        .map(|row| {
            row.try_get_by_index::<String>(0)
                .expect("first column is text")
        })
        .collect()
}

async fn one(db: &DatabaseConnection, sql: &str) -> String {
    let mut rows = strings(db, sql).await;
    assert_eq!(rows.len(), 1, "expected exactly one row for: {sql}");
    rows.remove(0)
}

fn unique() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn expected_tables() -> Vec<String> {
    let mut all: Vec<String> = TABLES
        .iter()
        .flat_map(|(schema, tables)| tables.iter().map(move |t| format!("{schema}.{t}")))
        .collect();
    all.sort();
    all
}

// ---------------------------------------------------------------- seed rows
// Minimal valid rows. References to another module's rows are just generated ids.

async fn term(db: &DatabaseConnection) -> String {
    one(
        db,
        &format!(
            "INSERT INTO calendar.terms (name, starts_at, ends_at)
             VALUES ('term-{}', '2026-08-01 00:00+00', '2026-12-01 00:00+00') RETURNING id::text",
            unique()
        ),
    )
    .await
}

fn user_sql(status: &str) -> String {
    format!(
        "INSERT INTO identity.users (email, full_name, password_hash, status)
         VALUES ('{}@example.com', 'Test User', 'hash', '{status}') RETURNING id::text",
        unique()
    )
}

async fn user(db: &DatabaseConnection) -> String {
    one(db, &user_sql("ACTIVE")).await
}

fn device_sql(user: &str, term: &str, status: &str, platform: &str) -> String {
    format!(
        "INSERT INTO identity.devices (user_id, term_id, public_key, platform, status)
         VALUES ('{user}', '{term}', 'key-{}', '{platform}', '{status}') RETURNING id::text",
        unique()
    )
}

async fn device(db: &DatabaseConnection) -> String {
    let (u, t) = (user(db).await, term(db).await);
    one(db, &device_sql(&u, &t, "ACTIVE", "IOS")).await
}

async fn course(db: &DatabaseConnection) -> String {
    one(
        db,
        &format!(
            "INSERT INTO courses.courses (term_id, code, title)
             VALUES (gen_random_uuid(), 'C{}', 'A course') RETURNING id::text",
            unique()
        ),
    )
    .await
}

async fn room(db: &DatabaseConnection) -> String {
    one(
        db,
        &format!(
            "INSERT INTO capture.rooms (building, room_code)
             VALUES ('Building', 'R{}') RETURNING id::text",
            unique()
        ),
    )
    .await
}

fn tag_sql(room: &str, status: &str) -> String {
    format!(
        "INSERT INTO capture.nfc_tags (tag_uid, room_id, status)
         VALUES ('tag-{}', '{room}', '{status}') RETURNING id::text",
        unique()
    )
}

async fn tag(db: &DatabaseConnection) -> String {
    let r = room(db).await;
    one(db, &tag_sql(&r, "ACTIVE")).await
}

fn session_sql(room: &str, start: &str, end: &str, cancelled: bool) -> String {
    let cancelled_at = if cancelled {
        "'2026-10-01 00:00+00'"
    } else {
        "NULL"
    };
    format!(
        "INSERT INTO capture.class_sessions
             (course_id, room_id, starts_at, ends_at, cancelled_at, created_by)
         VALUES (gen_random_uuid(), '{room}', '{start}', '{end}', {cancelled_at},
                 gen_random_uuid())
         RETURNING id::text"
    )
}

fn scan_sql(device: &str, tag: &str, counter: i32, result: &str, source: &str) -> String {
    format!(
        "INSERT INTO capture.scan_events
             (device_id, tag_id, tag_counter, captured_at, received_at, source, result)
         VALUES ('{device}', '{tag}', {counter}, now(), now(), '{source}', '{result}')
         RETURNING id::text"
    )
}

fn record_sql(session: &str, student: &str, status: &str) -> String {
    format!(
        "INSERT INTO records.attendance_records (session_id, student_id, status)
         VALUES ('{session}', '{student}', '{status}') RETURNING id::text"
    )
}

async fn record(db: &DatabaseConnection) -> String {
    one(
        db,
        &record_sql(
            &uuid::Uuid::new_v4().to_string(),
            &uuid::Uuid::new_v4().to_string(),
            "ABSENT",
        ),
    )
    .await
}

fn appeal_sql(record: &str, status: &str) -> String {
    format!(
        "INSERT INTO records.appeals (record_id, reason, status)
         VALUES ('{record}', 'My check-out failed', '{status}') RETURNING id::text"
    )
}

// ----------------------------------------------------- structure of the schema

#[tokio::test]
#[serial]
async fn every_table_lives_in_its_modules_schema() {
    let db = db().await;
    let found = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name FROM information_schema.tables
             WHERE table_type = 'BASE TABLE' AND table_schema IN {SCHEMAS} ORDER BY 1"
        ),
    )
    .await;
    assert_eq!(found, expected_tables());

    let public = strings(
        &db,
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY 1",
    )
    .await;
    assert_eq!(
        public,
        vec!["seaql_migrations"],
        "no module table may live in `public`"
    );
}

#[tokio::test]
#[serial]
async fn primary_keys_are_uuids_with_a_database_default() {
    let db = db().await;

    let keys = strings(
        &db,
        &format!(
            "SELECT tc.table_schema || '.' || tc.table_name || ' ' ||
                    string_agg(kcu.column_name, ',' ORDER BY kcu.ordinal_position)
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON kcu.constraint_name = tc.constraint_name
              AND kcu.table_schema = tc.table_schema
              AND kcu.table_name = tc.table_name
             WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema IN {SCHEMAS}
             GROUP BY tc.table_schema, tc.table_name ORDER BY 1"
        ),
    )
    .await;
    let mut expected: Vec<String> = expected_tables()
        .into_iter()
        .map(|t| {
            if t == "courses.course_members" {
                format!("{t} course_id,user_id")
            } else {
                format!("{t} id")
            }
        })
        .collect();
    expected.sort();
    assert_eq!(keys, expected);

    let uuid_ids = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name FROM information_schema.columns
             WHERE column_name = 'id' AND data_type = 'uuid'
               AND column_default = 'gen_random_uuid()' AND table_schema IN {SCHEMAS} ORDER BY 1"
        ),
    )
    .await;
    let mut expected_ids = expected_tables();
    expected_ids.retain(|t| t != "courses.course_members");
    assert_eq!(uuid_ids, expected_ids);
}

#[tokio::test]
#[serial]
async fn timestamp_columns_follow_the_agreed_pattern() {
    let db = db().await;

    let found = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name || '.' || column_name
             FROM information_schema.columns
             WHERE column_name IN ('created_at', 'updated_at') AND table_schema IN {SCHEMAS}
               AND data_type = 'timestamp with time zone' AND is_nullable = 'NO'
               AND column_default = 'now()' ORDER BY 1"
        ),
    )
    .await;
    let mut expected: Vec<String> = Vec::new();
    for table in expected_tables() {
        expected.push(format!("{table}.created_at"));
        if !CREATED_AT_ONLY.contains(&table.as_str()) {
            expected.push(format!("{table}.updated_at"));
        }
    }
    expected.sort();
    assert_eq!(found, expected);

    let naive = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name || '.' || column_name
             FROM information_schema.columns
             WHERE data_type = 'timestamp without time zone' AND table_schema IN {SCHEMAS}"
        ),
    )
    .await;
    assert!(
        naive.is_empty(),
        "timestamps must carry a time zone: {naive:?}"
    );
}

#[tokio::test]
#[serial]
async fn updated_at_is_set_by_the_database_when_a_row_changes() {
    let db = db().await;

    let with_trigger = strings(
        &db,
        "SELECT event_object_schema || '.' || event_object_table
         FROM information_schema.triggers WHERE trigger_name = 'set_updated_at' ORDER BY 1",
    )
    .await;
    let mut expected: Vec<String> = expected_tables();
    expected.retain(|t| !CREATED_AT_ONLY.contains(&t.as_str()));
    assert_eq!(with_trigger, expected);

    let id = term(&db).await;
    ok(&db, "SELECT pg_sleep(0.05)").await;
    ok(
        &db,
        &format!(
            "UPDATE calendar.terms SET name = 'renamed-{}' WHERE id = '{id}'",
            unique()
        ),
    )
    .await;
    let moved = one(
        &db,
        &format!("SELECT (updated_at > created_at)::text FROM calendar.terms WHERE id = '{id}'"),
    )
    .await;
    assert_eq!(moved, "true");
}

#[tokio::test]
#[serial]
async fn foreign_keys_exist_inside_modules_and_never_across_them() {
    let db = db().await;
    let found = strings(
        &db,
        &format!(
            "SELECT n1.nspname || '.' || c1.relname || '.' || a.attname || ' -> ' ||
                    n2.nspname || '.' || c2.relname
             FROM pg_constraint con
             JOIN pg_class c1 ON c1.oid = con.conrelid
             JOIN pg_namespace n1 ON n1.oid = c1.relnamespace
             JOIN pg_class c2 ON c2.oid = con.confrelid
             JOIN pg_namespace n2 ON n2.oid = c2.relnamespace
             JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = ANY (con.conkey)
             WHERE con.contype = 'f' AND n1.nspname IN {SCHEMAS} ORDER BY 1"
        ),
    )
    .await;
    let mut expected = vec![
        "identity.devices.user_id -> identity.users",
        "identity.device_lifecycle_items.device_id -> identity.devices",
        "courses.course_members.course_id -> courses.courses",
        "capture.nfc_tags.room_id -> capture.rooms",
        "capture.class_sessions.room_id -> capture.rooms",
        "capture.scan_events.tag_id -> capture.nfc_tags",
        "capture.scan_events.session_id -> capture.class_sessions",
        "records.attendance_record_lifecycle_items.record_id -> records.attendance_records",
        "records.attendance_record_lifecycle_items.appeal_id -> records.appeals",
        "records.appeals.record_id -> records.attendance_records",
        "records.appeal_lifecycle_items.appeal_id -> records.appeals",
    ];
    expected.sort_unstable();
    assert_eq!(found, expected);

    for line in &found {
        let (from, to) = line.split_once(" -> ").unwrap();
        let from_schema = from.split('.').next().unwrap();
        let to_schema = to.split('.').next().unwrap();
        assert_eq!(from_schema, to_schema, "cross-module foreign key: {line}");
    }
}

#[tokio::test]
#[serial]
async fn migrations_are_forward_only() {
    // 1. No migration ships a down script.
    let mut stack = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src"
    ))];
    let mut down_scripts = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.to_string_lossy().ends_with(".down.sql") {
                down_scripts.push(path.display().to_string());
            }
        }
    }
    assert!(
        down_scripts.is_empty(),
        "migrations are forward-only, remove: {down_scripts:?}"
    );

    // 2. Asking for a revert fails and leaves the database exactly as it was.
    let db = db().await;
    let before = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name FROM information_schema.tables
             WHERE table_type = 'BASE TABLE' AND table_schema IN {SCHEMAS} ORDER BY 1"
        ),
    )
    .await;
    assert_eq!(before, expected_tables());
    assert!(
        Migrator::down(&db, Some(1)).await.is_err(),
        "a revert must be refused"
    );
    let after = strings(
        &db,
        &format!(
            "SELECT table_schema || '.' || table_name FROM information_schema.tables
             WHERE table_type = 'BASE TABLE' AND table_schema IN {SCHEMAS} ORDER BY 1"
        ),
    )
    .await;
    assert_eq!(after, before, "a refused revert must not change the schema");
}

// ------------------------------------------------------------------ value sets

#[tokio::test]
#[serial]
async fn status_like_columns_only_accept_their_value_sets() {
    let db = db().await;

    // identity.users.status
    for status in ["INVITED", "ACTIVE", "SUSPENDED"] {
        ok(&db, &user_sql(status)).await;
    }
    rejected(&db, &user_sql("DELETED"), CHECK).await;

    // courses.course_members.role and status
    let course_id = course(&db).await;
    let member = |user: &str, role: &str, status: &str| {
        format!(
            "INSERT INTO courses.course_members (course_id, user_id, role, status)
             VALUES ('{course_id}', '{user}', '{role}', '{status}')"
        )
    };
    for role in ["STUDENT", "TA", "PROFESSOR"] {
        ok(
            &db,
            &member(&uuid::Uuid::new_v4().to_string(), role, "ACTIVE"),
        )
        .await;
    }
    for status in ["ACTIVE", "REMOVED"] {
        ok(
            &db,
            &member(&uuid::Uuid::new_v4().to_string(), "STUDENT", status),
        )
        .await;
    }
    rejected(
        &db,
        &member(&uuid::Uuid::new_v4().to_string(), "ADMIN", "ACTIVE"),
        CHECK,
    )
    .await;
    rejected(
        &db,
        &member(&uuid::Uuid::new_v4().to_string(), "STUDENT", "GONE"),
        CHECK,
    )
    .await;

    // capture.nfc_tags.status
    let room_id = room(&db).await;
    for status in ["ACTIVE", "REVOKED"] {
        ok(&db, &tag_sql(&room_id, status)).await;
    }
    rejected(&db, &tag_sql(&room_id, "LOST"), CHECK).await;

    // identity.devices.platform and status
    let (u, t) = (user(&db).await, term(&db).await);
    ok(&db, &device_sql(&u, &t, "REVOKED", "IOS")).await;
    ok(&db, &device_sql(&u, &t, "REVOKED", "ANDROID")).await;
    rejected(&db, &device_sql(&u, &t, "REVOKED", "WINDOWS"), CHECK).await;
    rejected(&db, &device_sql(&u, &t, "BROKEN", "IOS"), CHECK).await;

    // capture.scan_events.source and result
    let (d, tg) = (device(&db).await, tag(&db).await);
    ok(&db, &scan_sql(&d, &tg, 1, "ACCEPTED", "ONLINE")).await;
    ok(&db, &scan_sql(&d, &tg, 2, "REJECTED", "OFFLINE")).await;
    rejected(
        &db,
        &scan_sql(&d, &tg, 3, "ACCEPTED", "CARRIER_PIGEON"),
        CHECK,
    )
    .await;
    rejected(&db, &scan_sql(&d, &tg, 4, "MAYBE", "ONLINE"), CHECK).await;

    // records.attendance_records.status
    for status in [
        "CHECKED_IN",
        "PRESENT",
        "LATE",
        "INCOMPLETE",
        "ABSENT",
        "EXCUSED",
    ] {
        ok(
            &db,
            &record_sql(
                &uuid::Uuid::new_v4().to_string(),
                &uuid::Uuid::new_v4().to_string(),
                status,
            ),
        )
        .await;
    }
    rejected(
        &db,
        &record_sql(
            &uuid::Uuid::new_v4().to_string(),
            &uuid::Uuid::new_v4().to_string(),
            "SKIPPED",
        ),
        CHECK,
    )
    .await;

    // records.appeals.status
    let rec = record(&db).await;
    ok(&db, &appeal_sql(&rec, "APPROVED")).await;
    rejected(&db, &appeal_sql(&rec, "WITHDRAWN"), CHECK).await;
}

// ----------------------------------------------------------- uniqueness rules

#[tokio::test]
#[serial]
async fn natural_keys_are_unique() {
    let db = db().await;

    ok(
        &db,
        "INSERT INTO calendar.terms (name, starts_at, ends_at)
             VALUES ('AY2026', now(), now() + interval '1 day')",
    )
    .await;
    rejected(
        &db,
        "INSERT INTO calendar.terms (name, starts_at, ends_at)
                   VALUES ('AY2026', now(), now() + interval '1 day')",
        UNIQUE,
    )
    .await;

    ok(
        &db,
        "INSERT INTO identity.users (email, full_name, password_hash, status)
             VALUES ('same@example.com', 'A', 'h', 'ACTIVE')",
    )
    .await;
    rejected(
        &db,
        "INSERT INTO identity.users (email, full_name, password_hash, status)
                   VALUES ('same@example.com', 'B', 'h', 'ACTIVE')",
        UNIQUE,
    )
    .await;

    let (u, t) = (user(&db).await, term(&db).await);
    ok(
        &db,
        &format!(
            "INSERT INTO identity.devices (user_id, term_id, public_key, platform, status)
                      VALUES ('{u}', '{t}', 'dup-key', 'IOS', 'REVOKED')"
        ),
    )
    .await;
    rejected(
        &db,
        &format!(
            "INSERT INTO identity.devices (user_id, term_id, public_key, platform, status)
                            VALUES ('{u}', '{t}', 'dup-key', 'IOS', 'REVOKED')"
        ),
        UNIQUE,
    )
    .await;

    let r = room(&db).await;
    ok(&db, &format!("INSERT INTO capture.nfc_tags (tag_uid, room_id, status) VALUES ('uid-1', '{r}', 'ACTIVE')")).await;
    rejected(&db, &format!("INSERT INTO capture.nfc_tags (tag_uid, room_id, status) VALUES ('uid-1', '{r}', 'ACTIVE')"), UNIQUE).await;

    // A room is identified by its building and code together.
    ok(
        &db,
        "INSERT INTO capture.rooms (building, room_code) VALUES ('SUTD', 'LT1')",
    )
    .await;
    rejected(
        &db,
        "INSERT INTO capture.rooms (building, room_code) VALUES ('SUTD', 'LT1')",
        UNIQUE,
    )
    .await;
    ok(
        &db,
        "INSERT INTO capture.rooms (building, room_code) VALUES ('SUTD', 'LT2')",
    )
    .await;
    ok(
        &db,
        "INSERT INTO capture.rooms (building, room_code) VALUES ('Other', 'LT1')",
    )
    .await;

    // A course code is unique within a term, and free to repeat in another term.
    let course_in = |term: &str, code: &str| {
        format!("INSERT INTO courses.courses (term_id, code, title) VALUES ('{term}', '{code}', 'A course')")
    };
    let (term_a, term_b) = (
        uuid::Uuid::new_v4().to_string(),
        uuid::Uuid::new_v4().to_string(),
    );
    ok(&db, &course_in(&term_a, "50.057")).await;
    rejected(&db, &course_in(&term_a, "50.057"), UNIQUE).await;
    ok(&db, &course_in(&term_b, "50.057")).await;
    ok(&db, &course_in(&term_a, "50.058")).await;

    let c = course(&db).await;
    let member = format!(
        "INSERT INTO courses.course_members (course_id, user_id, role, status)
                          VALUES ('{c}', '{u}', 'STUDENT', 'ACTIVE')"
    );
    ok(&db, &member).await;
    rejected(&db, &member, UNIQUE).await;
}

/// An insert into `identity.users` with this SQL expression as the email.
fn user_with_email_sql(email_expr: &str) -> String {
    format!(
        "INSERT INTO identity.users (email, full_name, password_hash, status)
         VALUES ({email_expr}, 'Test User', 'hash', 'ACTIVE')"
    )
}

#[tokio::test]
#[serial]
async fn user_emails_are_stored_trimmed_and_lower_case() {
    let db = db().await;
    for email in [
        "'Alex@Example.edu'",
        "' alex@example.edu'",
        "'alex@example.edu '",
        "E'\\talex@example.edu'",
        "E'alex@example.edu\\n'",
    ] {
        rejected(&db, &user_with_email_sql(email), CHECK).await;
    }
    ok(&db, &user_with_email_sql("'alex@example.edu'")).await;
}

/// The code trims with `str::trim`; the constraint must treat exactly the same characters as
/// whitespace, or an email the code accepts could be refused by the database (or the reverse).
#[tokio::test]
#[serial]
async fn the_email_constraint_trims_the_same_characters_as_the_code() {
    let db = db().await;
    for c in (0..=0xFFFF_u32)
        .filter_map(char::from_u32)
        .filter(|c| c.is_whitespace())
    {
        let code = u32::from(c);
        rejected(
            &db,
            &user_with_email_sql(&format!("E'\\u{code:04x}alex@example.edu'")),
            CHECK,
        )
        .await;
        rejected(
            &db,
            &user_with_email_sql(&format!("E'alex@example.edu\\u{code:04x}'")),
            CHECK,
        )
        .await;
    }
    // Whitespace inside the address is not trimmed, so the constraint leaves it alone.
    ok(&db, &user_with_email_sql("'a b@example.edu'")).await;
}

#[tokio::test]
#[serial]
async fn a_user_can_have_only_one_active_device_per_term() {
    let db = db().await;
    let (u, t) = (user(&db).await, term(&db).await);
    let other_term = term(&db).await;

    ok(&db, &device_sql(&u, &t, "ACTIVE", "IOS")).await;
    rejected(&db, &device_sql(&u, &t, "ACTIVE", "ANDROID"), UNIQUE).await;
    // A replaced (revoked) device does not count, and another term is independent.
    ok(&db, &device_sql(&u, &t, "REVOKED", "IOS")).await;
    ok(&db, &device_sql(&u, &other_term, "ACTIVE", "IOS")).await;
}

// ------------------------------------------------------------------- sessions

#[tokio::test]
#[serial]
async fn a_session_must_end_after_it_starts() {
    let db = db().await;
    let r = room(&db).await;
    rejected(
        &db,
        &session_sql(&r, "2026-10-20 12:00+00", "2026-10-20 10:00+00", false),
        CHECK,
    )
    .await;
    rejected(
        &db,
        &session_sql(&r, "2026-10-20 10:00+00", "2026-10-20 10:00+00", false),
        CHECK,
    )
    .await;
    ok(
        &db,
        &session_sql(&r, "2026-10-20 10:00+00", "2026-10-20 12:00+00", false),
    )
    .await;
}

#[tokio::test]
#[serial]
async fn sessions_cannot_overlap_in_a_room_unless_cancelled() {
    let db = db().await;
    let r = room(&db).await;
    ok(
        &db,
        &session_sql(&r, "2026-10-20 10:00+00", "2026-10-20 12:00+00", false),
    )
    .await;

    // Overlap in the same room is rejected.
    rejected(
        &db,
        &session_sql(&r, "2026-10-20 11:00+00", "2026-10-20 13:00+00", false),
        EXCLUSION,
    )
    .await;
    // Back to back is fine: the end of one is the start of the next.
    ok(
        &db,
        &session_sql(&r, "2026-10-20 12:00+00", "2026-10-20 14:00+00", false),
    )
    .await;
    // A different room is independent.
    let other = room(&db).await;
    ok(
        &db,
        &session_sql(&other, "2026-10-20 11:00+00", "2026-10-20 13:00+00", false),
    )
    .await;
    // A cancelled session does not block the room, and does not block others either.
    ok(
        &db,
        &session_sql(&r, "2026-10-20 15:00+00", "2026-10-20 17:00+00", true),
    )
    .await;
    ok(
        &db,
        &session_sql(&r, "2026-10-20 15:00+00", "2026-10-20 17:00+00", false),
    )
    .await;
}

// ---------------------------------------------------------------- scan events

#[tokio::test]
#[serial]
async fn scan_events_follow_the_uniqueness_rules() {
    let db = db().await;
    let tg = tag(&db).await;
    let (d1, d2) = (device(&db).await, device(&db).await);

    // The same tap by the same device is stored once.
    ok(&db, &scan_sql(&d1, &tg, 7, "ACCEPTED", "ONLINE")).await;
    rejected(&db, &scan_sql(&d1, &tg, 7, "ACCEPTED", "ONLINE"), UNIQUE).await;
    rejected(&db, &scan_sql(&d1, &tg, 7, "REJECTED", "ONLINE"), UNIQUE).await;

    // A tap number counts as accepted only once across devices.
    rejected(&db, &scan_sql(&d2, &tg, 7, "ACCEPTED", "ONLINE"), UNIQUE).await;
    // But another device's rejected attempt with that number is just history.
    ok(&db, &scan_sql(&d2, &tg, 7, "REJECTED", "ONLINE")).await;
    // And a different tap number is independent.
    ok(&db, &scan_sql(&d2, &tg, 8, "ACCEPTED", "ONLINE")).await;
}

// ------------------------------------------------------------ records, appeals

#[tokio::test]
#[serial]
async fn a_student_has_one_record_per_session() {
    let db = db().await;
    let (session, student) = (
        uuid::Uuid::new_v4().to_string(),
        uuid::Uuid::new_v4().to_string(),
    );
    ok(&db, &record_sql(&session, &student, "ABSENT")).await;
    rejected(&db, &record_sql(&session, &student, "PRESENT"), UNIQUE).await;
    ok(
        &db,
        &record_sql(&session, &uuid::Uuid::new_v4().to_string(), "ABSENT"),
    )
    .await;
    ok(
        &db,
        &record_sql(&uuid::Uuid::new_v4().to_string(), &student, "ABSENT"),
    )
    .await;
}

#[tokio::test]
#[serial]
async fn a_record_has_at_most_one_pending_appeal_but_can_be_appealed_again() {
    let db = db().await;
    let rec = record(&db).await;

    ok(&db, &appeal_sql(&rec, "PENDING")).await;
    rejected(&db, &appeal_sql(&rec, "PENDING"), UNIQUE).await;
    // Decided appeals do not count, so a student can appeal again after a rejection.
    ok(&db, &appeal_sql(&rec, "REJECTED")).await;
    ok(&db, &appeal_sql(&rec, "APPROVED")).await;
    // Another record is independent.
    let other = record(&db).await;
    ok(&db, &appeal_sql(&other, "PENDING")).await;
}

#[tokio::test]
#[serial]
async fn appeals_store_supporting_documents_as_a_list_that_defaults_to_empty() {
    let db = db().await;
    let rec = record(&db).await;
    let id = one(&db, &appeal_sql(&rec, "PENDING")).await;
    let empty = one(
        &db,
        &format!(
            "SELECT cardinality(supporting_documents)::text FROM records.appeals WHERE id = '{id}'"
        ),
    )
    .await;
    assert_eq!(empty, "0");
    ok(&db, &format!("UPDATE records.appeals SET supporting_documents = ARRAY['a.pdf', 'b.png'] WHERE id = '{id}'")).await;
    let two = one(
        &db,
        &format!(
            "SELECT cardinality(supporting_documents)::text FROM records.appeals WHERE id = '{id}'"
        ),
    )
    .await;
    assert_eq!(two, "2");
}

// ------------------------------------------------------------- lifecycle rows

#[tokio::test]
#[serial]
async fn lifecycle_rows_must_change_the_status_and_use_known_statuses() {
    let db = db().await;

    let dev = device(&db).await;
    let device_row = |old: &str, new: &str| {
        format!("INSERT INTO identity.device_lifecycle_items (device_id, old_status, new_status, reason)
                 VALUES ('{dev}', {old}, '{new}', 'rebound')")
    };
    ok(&db, &device_row("NULL", "ACTIVE")).await; // the first row has no old status
    ok(&db, &device_row("'ACTIVE'", "REVOKED")).await;
    rejected(&db, &device_row("'ACTIVE'", "ACTIVE"), CHECK).await;
    rejected(&db, &device_row("NULL", "BROKEN"), CHECK).await;

    let rec = record(&db).await;
    let record_row = |old: &str, new: &str| {
        format!("INSERT INTO records.attendance_record_lifecycle_items (record_id, old_status, new_status, reason)
                 VALUES ('{rec}', {old}, '{new}', 'tap')")
    };
    ok(&db, &record_row("NULL", "CHECKED_IN")).await;
    ok(&db, &record_row("'CHECKED_IN'", "PRESENT")).await;
    rejected(&db, &record_row("'PRESENT'", "PRESENT"), CHECK).await;
    rejected(&db, &record_row("NULL", "SKIPPED"), CHECK).await;

    let appeal = one(&db, &appeal_sql(&rec, "PENDING")).await;
    let appeal_row = |old: &str, new: &str| {
        format!("INSERT INTO records.appeal_lifecycle_items (appeal_id, old_status, new_status, reason, changed_by)
                 VALUES ('{appeal}', {old}, '{new}', 'decision', gen_random_uuid())")
    };
    ok(&db, &appeal_row("NULL", "PENDING")).await;
    ok(&db, &appeal_row("'PENDING'", "APPROVED")).await;
    rejected(&db, &appeal_row("'APPROVED'", "APPROVED"), CHECK).await;
    rejected(&db, &appeal_row("NULL", "WITHDRAWN"), CHECK).await;

    // A reason is optional context on every lifecycle row.
    ok(
        &db,
        &format!(
            "INSERT INTO identity.device_lifecycle_items (device_id, new_status)
             VALUES ('{dev}', 'ACTIVE')"
        ),
    )
    .await;
    ok(
        &db,
        &format!(
            "INSERT INTO records.attendance_record_lifecycle_items (record_id, new_status)
             VALUES ('{rec}', 'PRESENT')"
        ),
    )
    .await;
    ok(
        &db,
        &format!(
            "INSERT INTO records.appeal_lifecycle_items (appeal_id, new_status, changed_by)
             VALUES ('{appeal}', 'APPROVED', gen_random_uuid())"
        ),
    )
    .await;
    // But an appeal change always names the person who made it.
    rejected(
        &db,
        &format!(
            "INSERT INTO records.appeal_lifecycle_items (appeal_id, new_status, reason)
             VALUES ('{appeal}', 'APPROVED', 'x')"
        ),
        "null value",
    )
    .await;
}

#[tokio::test]
#[serial]
async fn a_record_change_has_at_most_one_cause() {
    let db = db().await;
    let rec = record(&db).await;
    let appeal = one(&db, &appeal_sql(&rec, "APPROVED")).await;
    let scan = uuid::Uuid::new_v4();
    let row = |scan: &str, appeal: &str, changed_by: &str| {
        format!("INSERT INTO records.attendance_record_lifecycle_items
                     (record_id, old_status, new_status, reason, scan_event_id, appeal_id, changed_by)
                 VALUES ('{rec}', 'ABSENT', 'PRESENT', 'why', {scan}, {appeal}, {changed_by})")
    };
    let scan_id = format!("'{scan}'");
    let appeal_id = format!("'{appeal}'");
    let person = "gen_random_uuid()";

    ok(&db, &row(&scan_id, "NULL", "NULL")).await; // caused by a scan (system)
    ok(&db, &row("NULL", &appeal_id, person)).await; // caused by an appeal
    ok(&db, &row("NULL", "NULL", person)).await; // a manual change
    rejected(&db, &row(&scan_id, &appeal_id, person), CHECK).await; // two causes
}
