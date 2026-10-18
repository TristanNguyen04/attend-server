CREATE TABLE records.attendance_records (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id uuid        NOT NULL, -- capture.class_sessions: ID only
    student_id uuid        NOT NULL, -- identity.users: ID only
    status     text        NOT NULL
        CHECK (status IN ('CHECKED_IN', 'PRESENT', 'LATE', 'INCOMPLETE', 'ABSENT', 'EXCUSED')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT attendance_records_one_per_session_and_student UNIQUE (session_id, student_id)
);
CREATE INDEX attendance_records_student_id ON records.attendance_records (student_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON records.attendance_records
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE records.appeals (
    id                   uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    record_id            uuid        NOT NULL REFERENCES records.attendance_records (id),
    reason               text        NOT NULL,
    status               text        NOT NULL CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED')),
    supporting_documents text[]      NOT NULL DEFAULT '{}',
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now()
);
-- At most one open appeal per record; a student may appeal again after a decision.
CREATE UNIQUE INDEX appeals_one_pending_per_record
    ON records.appeals (record_id) WHERE status = 'PENDING';
CREATE INDEX appeals_record_id ON records.appeals (record_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON records.appeals
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

-- Append-only history of record status changes: created_at only, no updated_at.
CREATE TABLE records.attendance_record_lifecycle_items (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    record_id     uuid        NOT NULL REFERENCES records.attendance_records (id),
    old_status    text
        CHECK (old_status IN ('CHECKED_IN', 'PRESENT', 'LATE', 'INCOMPLETE', 'ABSENT', 'EXCUSED')),
    new_status    text        NOT NULL
        CHECK (new_status IN ('CHECKED_IN', 'PRESENT', 'LATE', 'INCOMPLETE', 'ABSENT', 'EXCUSED')),
    reason        text,
    changed_by    uuid,       -- identity.users: ID only; null = the system
    scan_event_id uuid,       -- capture.scan_events: ID only
    appeal_id     uuid        REFERENCES records.appeals (id),
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT attendance_record_lifecycle_items_status_changes
        CHECK (old_status IS NULL OR new_status <> old_status),
    -- A change has at most one cause: a scan, or an appeal, or neither (a manual change).
    CONSTRAINT attendance_record_lifecycle_items_single_cause
        CHECK (num_nonnulls(scan_event_id, appeal_id) <= 1)
);
CREATE INDEX attendance_record_lifecycle_items_record_id
    ON records.attendance_record_lifecycle_items (record_id);

-- Append-only history of appeal status changes: created_at only, no updated_at.
CREATE TABLE records.appeal_lifecycle_items (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    appeal_id  uuid        NOT NULL REFERENCES records.appeals (id),
    old_status text        CHECK (old_status IN ('PENDING', 'APPROVED', 'REJECTED')),
    new_status text        NOT NULL CHECK (new_status IN ('PENDING', 'APPROVED', 'REJECTED')),
    reason     text,
    changed_by uuid        NOT NULL, -- identity.users: ID only
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT appeal_lifecycle_items_status_changes
        CHECK (old_status IS NULL OR new_status <> old_status)
);
CREATE INDEX appeal_lifecycle_items_appeal_id ON records.appeal_lifecycle_items (appeal_id);
