-- Add migration script here
CREATE TABLE thread (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc'),
    club_id INTEGER NOT NULL,
    likes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (club_id) REFERENCES club(id)
);