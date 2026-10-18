# Data model

Approved ER diagram. Each table belongs to one module and lives in that module's Postgres
schema (see `docs/architecture.md`). References across modules are plain IDs.

Every table also has `created_at` and `updated_at` (`timestamptz`), except the two
lifecycle tables below, which are append-only and have only `created_at`:
`device_lifecycle_items` and `attendance_record_lifecycle_items`.

```mermaid
erDiagram
  TERMS ||--o{ COURSES : "runs in"
  TERMS ||--o{ DEVICES : "bound for"
  USERS ||--o{ COURSE_MEMBERS : "is a member"
  COURSES ||--o{ COURSE_MEMBERS : has
  COURSES ||--o{ CLASS_SESSIONS : schedules
  ROOMS ||--o{ CLASS_SESSIONS : hosts
  ROOMS ||--o{ NFC_TAGS : "has tags"
  USERS ||--o{ DEVICES : binds
  DEVICES ||--o{ DEVICE_LIFECYCLE_ITEMS : "status history"
  DEVICES ||--o{ SCAN_EVENTS : signs
  NFC_TAGS ||--o{ SCAN_EVENTS : "scanned in"
  CLASS_SESSIONS ||--o{ SCAN_EVENTS : receives
  CLASS_SESSIONS ||--o{ ATTENDANCE_RECORDS : has
  USERS ||--o{ ATTENDANCE_RECORDS : "student of"
  ATTENDANCE_RECORDS ||--o{ ATTENDANCE_RECORD_LIFECYCLE_ITEMS : "status history"
  SCAN_EVENTS ||--o{ ATTENDANCE_RECORD_LIFECYCLE_ITEMS : "caused by"
  APPEALS ||--o{ ATTENDANCE_RECORD_LIFECYCLE_ITEMS : "applied as"
  USERS ||--o{ ATTENDANCE_RECORD_LIFECYCLE_ITEMS : "changed by"
  ATTENDANCE_RECORDS ||--o{ APPEALS : "appealed by"
  APPEALS ||--o{ APPEAL_LIFECYCLE_ITEMS : "status history"
  USERS ||--o{ APPEAL_LIFECYCLE_ITEMS : "changed by"

  TERMS { uuid id PK
          string name UK
          timestamptz starts_at
          timestamptz ends_at }
  USERS { uuid id PK
          string email UK
          string full_name
          string password_hash
          string status "INVITED | ACTIVE | SUSPENDED" }
  COURSES { uuid id PK
            uuid term_id FK
            string code
            string title
            jsonb policies }
  COURSE_MEMBERS { uuid course_id PK,FK
                   uuid user_id PK,FK
                   string role "STUDENT | TA | PROFESSOR"
                   string status "ACTIVE | REMOVED" }
  ROOMS { uuid id PK
          string building
          string room_code }
  NFC_TAGS { uuid id PK
             string tag_uid UK
             uuid room_id FK
             string status "ACTIVE | REVOKED" }
  CLASS_SESSIONS { uuid id PK
                   uuid course_id FK
                   uuid room_id FK
                   timestamptz starts_at
                   timestamptz ends_at
                   timestamptz closed_at "early close"
                   timestamptz cancelled_at
                   uuid created_by FK }
  DEVICES { uuid id PK
            uuid user_id FK
            uuid term_id FK
            string public_key UK
            string platform "IOS | ANDROID"
            string status "ACTIVE | REVOKED"
            timestamptz attested_at "nullable" }
  DEVICE_LIFECYCLE_ITEMS { uuid id PK
                           uuid device_id FK
                           string old_status "null on first row"
                           string new_status
                           string reason
                           timestamptz created_at }
  SCAN_EVENTS { uuid id PK
                uuid device_id FK
                uuid tag_id FK
                uuid session_id FK "null if none matched"
                int tag_counter
                timestamptz captured_at "phone clock"
                timestamptz received_at "server clock"
                string source "ONLINE | OFFLINE"
                string result "ACCEPTED | REJECTED"
                string reason }
  ATTENDANCE_RECORDS { uuid id PK
                       uuid session_id FK
                       uuid student_id FK
                       string status "CHECKED_IN | PRESENT | LATE | INCOMPLETE | ABSENT | EXCUSED" }
  ATTENDANCE_RECORD_LIFECYCLE_ITEMS { uuid id PK
                                      uuid record_id FK
                                      string old_status "null on first row"
                                      string new_status
                                      string reason
                                      uuid changed_by FK "null = system"
                                      uuid scan_event_id FK "nullable"
                                      uuid appeal_id FK "nullable"
                                      timestamptz created_at }
  APPEALS { uuid id PK
            uuid record_id FK
            string reason
            string status "PENDING | APPROVED | REJECTED"
            text_array supporting_documents }
  APPEAL_LIFECYCLE_ITEMS { uuid id PK
                           uuid appeal_id FK
                           string old_status "null on first row"
                           string new_status
                           string reason
                           uuid changed_by FK
                           timestamptz created_at }
```

Foreign key lines that cross a module boundary in the picture are **IDs only** in the
database (rule 6 in `docs/architecture.md`).

## Constraints

| Table | Constraint |
|---|---|
| `devices` | Unique `(user_id, term_id)` where status is ACTIVE: one phone per account per term. Rebinding needs the latest device to be at least 7 days old. |
| `scan_events` | Unique `(device_id, tag_id, tag_counter)`, and unique `(tag_id, tag_counter)` where result is ACCEPTED |
| `class_sessions` | `ends_at > starts_at`. No overlap per room among sessions with `cancelled_at` null. |
| `attendance_records` | Unique `(session_id, student_id)` |
| `appeals` | At most one PENDING appeal per record; a student may appeal again after a rejection |
| record lifecycle items | At most one of `scan_event_id` and `appeal_id` is set; a manual change needs `changed_by` |
| all lifecycle items | `new_status` differs from `old_status` |

## Derived values (not stored)

Session state comes from time, not a column:

```
CANCELLED  if cancelled_at is not null
CLOSED     if closed_at is not null, or now >= ends_at
OPEN       if now is between starts_at and coalesce(closed_at, ends_at)
SCHEDULED  otherwise
```

## Course policy (`courses.policies`)

```json
{ "late_after_minutes": 10, "absent_after_minutes": 30,
  "require_checkout": true, "earliest_checkout_after_minutes": 45 }
```

Policies cannot be changed while a session of the course is OPEN.

## Attendance status flow

| Event | Without checkout | With checkout |
|---|---|---|
| First tap | PRESENT or LATE | CHECKED_IN |
| Second tap | not applicable | PRESENT or LATE |
| Session ends still CHECKED_IN | not applicable | INCOMPLETE |
| No tap | ABSENT | ABSENT |
| Manual change or approved appeal | any status, with a reason | same |
