-- Add migration script here
CREATE TABLE likes (
    id      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    post_id UUID NOT NULL REFERENCES posts(id),
    UNIQUE (user_id, post_id)
);
