CREATE TABLE courses.courses (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id    uuid        NOT NULL, -- calendar.terms: ID only, no foreign key across modules
    code       text        NOT NULL,
    title      text        NOT NULL,
    policies   jsonb,      -- shape: shared_lib::policies::CoursePolicies; null = not configured yet
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    -- Also serves lookups by term: term_id is the first column of this unique index.
    CONSTRAINT courses_term_id_code_key UNIQUE (term_id, code)
);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON courses.courses
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TABLE courses.course_members (
    course_id  uuid        NOT NULL REFERENCES courses.courses (id),
    user_id    uuid        NOT NULL, -- identity.users: ID only, no foreign key across modules
    role       text        NOT NULL CHECK (role IN ('STUDENT', 'TA', 'PROFESSOR')),
    status     text        NOT NULL CHECK (status IN ('ACTIVE', 'REMOVED')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (course_id, user_id)
);
CREATE INDEX course_members_user_id ON courses.course_members (user_id);
CREATE TRIGGER set_updated_at BEFORE UPDATE ON courses.course_members
    FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();
