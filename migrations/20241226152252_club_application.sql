-- Add migration script here
CREATE TABLE club_application (
    id SERIAL PRIMARY KEY,
    club_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    message TEXT,
    accepted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc'),
    accepted_at TIMESTAMP WITH TIME ZONE,
    FOREIGN KEY (club_id) REFERENCES club(id),
    FOREIGN KEY (user_id) REFERENCES user_profile(id)
);