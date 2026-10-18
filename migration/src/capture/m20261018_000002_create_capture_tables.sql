-- Needed for the "no overlapping sessions in a room" exclusion constraint (equality on a uuid
-- inside a GiST index). The extension is database-wide and is left in place by `down`.
CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE capture.rooms (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    building   text        NOT NULL,
    room_code  text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT rooms_building_room_code_key UNIQUE (building, room_code)
);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON capture.rooms
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE capture.nfc_tags (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_uid    text        NOT NULL UNIQUE,
    room_id    uuid        NOT NULL REFERENCES capture.rooms (id),
    status     text        NOT NULL CHECK (status IN ('ACTIVE', 'REVOKED')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX nfc_tags_room_id ON capture.nfc_tags (room_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON capture.nfc_tags
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE capture.class_sessions (
    id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id    uuid        NOT NULL, -- courses.courses: ID only
    room_id      uuid        NOT NULL REFERENCES capture.rooms (id),
    starts_at    timestamptz NOT NULL,
    ends_at      timestamptz NOT NULL,
    closed_at    timestamptz, -- early close
    cancelled_at timestamptz,
    created_by   uuid        NOT NULL, -- identity.users: ID only
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT class_sessions_ends_after_start CHECK (ends_at > starts_at),
    -- No two live sessions may use a room at the same time. The range [starts_at, ends_at)
    -- is half-open, so a session may start exactly when the previous one ends.
    CONSTRAINT class_sessions_no_room_overlap
        EXCLUDE USING gist (room_id WITH =, tstzrange(starts_at, ends_at) WITH &&)
        WHERE (cancelled_at IS NULL)
);
CREATE INDEX class_sessions_course_id ON capture.class_sessions (course_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON capture.class_sessions
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE capture.scan_events (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id   uuid        NOT NULL, -- identity.devices: ID only
    tag_id      uuid        NOT NULL REFERENCES capture.nfc_tags (id),
    session_id  uuid        REFERENCES capture.class_sessions (id), -- null if none matched
    tag_counter integer     NOT NULL,
    captured_at timestamptz NOT NULL, -- phone clock
    received_at timestamptz NOT NULL, -- server clock
    source      text        NOT NULL CHECK (source IN ('ONLINE', 'OFFLINE')),
    result      text        NOT NULL CHECK (result IN ('ACCEPTED', 'REJECTED')),
    reason      text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    -- A retry of the same physical tap by the same phone is stored once.
    CONSTRAINT scan_events_device_tag_counter_key UNIQUE (device_id, tag_id, tag_counter)
);
-- Each tap number can be accepted only once across all phones.
CREATE UNIQUE INDEX scan_events_accepted_tag_counter
    ON capture.scan_events (tag_id, tag_counter) WHERE result = 'ACCEPTED';
CREATE INDEX scan_events_session_id ON capture.scan_events (session_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON capture.scan_events
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();
