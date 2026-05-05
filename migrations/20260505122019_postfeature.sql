-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";


CREATE TABLE posts (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    user_id     UUID NOT NULL REFERENCES users(id),
    likes_count BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
