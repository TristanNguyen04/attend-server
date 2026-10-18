CREATE TABLE identity.users (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text        NOT NULL UNIQUE,
    full_name     text        NOT NULL,
    password_hash text        NOT NULL,
    status        text        NOT NULL CHECK (status IN ('INVITED', 'ACTIVE', 'SUSPENDED')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON identity.users
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE identity.devices (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid        NOT NULL REFERENCES identity.users (id),
    term_id     uuid        NOT NULL, -- calendar.terms: ID only, no foreign key across modules
    public_key  text        NOT NULL UNIQUE,
    platform    text        NOT NULL CHECK (platform IN ('IOS', 'ANDROID')),
    status      text        NOT NULL CHECK (status IN ('ACTIVE', 'REVOKED')),
    attested_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
-- One active phone per account per term.
CREATE UNIQUE INDEX devices_one_active_per_user_and_term
    ON identity.devices (user_id, term_id) WHERE status = 'ACTIVE';
CREATE INDEX devices_user_id ON identity.devices (user_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON identity.devices
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

-- Append-only history of device status changes: created_at only, no updated_at.
CREATE TABLE identity.device_lifecycle_items (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id  uuid        NOT NULL REFERENCES identity.devices (id),
    old_status text        CHECK (old_status IN ('ACTIVE', 'REVOKED')),
    new_status text        NOT NULL CHECK (new_status IN ('ACTIVE', 'REVOKED')),
    reason     text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT device_lifecycle_items_status_changes
        CHECK (old_status IS NULL OR new_status <> old_status)
);
CREATE INDEX device_lifecycle_items_device_id ON identity.device_lifecycle_items (device_id);
