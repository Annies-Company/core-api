CREATE TABLE IF NOT EXISTS feedback (
    id          TEXT        PRIMARY KEY,
    status      TEXT        NOT NULL DEFAULT 'new'
                            CHECK (status IN ('new', 'read', 'published', 'hidden')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    rating      INTEGER     NOT NULL CHECK (rating BETWEEN 1 AND 5),
    name        TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    area        TEXT        NOT NULL DEFAULT '' CHECK (char_length(area) <= 80),
    phone       TEXT        NOT NULL DEFAULT '' CHECK (char_length(phone) <= 30),
    ordered     TEXT        NOT NULL DEFAULT '' CHECK (char_length(ordered) <= 200),
    message     TEXT        NOT NULL CHECK (char_length(message) BETWEEN 1 AND 3000),
    consent     BOOLEAN     NOT NULL,

    -- The promise made to the customer on the form, enforced by the
    -- database itself: a row without consent can never be 'published',
    -- no matter which code path (or future bug) tries.
    CONSTRAINT feedback_publish_needs_consent CHECK (status <> 'published' OR consent)
);

CREATE INDEX IF NOT EXISTS feedback_created_at_idx ON feedback (created_at DESC);

ALTER TABLE feedback ENABLE ROW LEVEL SECURITY;
