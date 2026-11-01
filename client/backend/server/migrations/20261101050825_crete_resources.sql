-- Add migration script here
CREATE TABLE resources (
    id            BIGSERIAL PRIMARY KEY,
    name          TEXT NOT NULL,
    category      TEXT NOT NULL CHECK (category IN ('imagery','terrain','tiles','vector')),
    url           TEXT,
    source_path   TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending','converting','ready','failed')),
    error_message TEXT,

    min_lon       DOUBLE PRECISION,
    min_lat       DOUBLE PRECISION,
    max_lon       DOUBLE PRECISION,
    max_lat       DOUBLE PRECISION,
    min_zoom      INTEGER,
    max_zoom      INTEGER,

    group_name         TEXT,
    extra         JSONB NOT NULL DEFAULT '{}',

    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_resources_category  ON resources (category);
CREATE INDEX idx_resources_group_name ON resources (group_name) WHERE group_name IS NOT NULL;