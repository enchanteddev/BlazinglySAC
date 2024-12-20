-- Add migration script here
-- Has user_id and club_id as conjugate primary keys

CREATE TABLE membership (
    user_id INTEGER NOT NULL,
    club_id INTEGER NOT NULL,
    role VARCHAR(255) NOT NULL,
    privilege_level INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc'),
    PRIMARY KEY (user_id, club_id),
    FOREIGN KEY (user_id) REFERENCES user_profile(id),
    FOREIGN KEY (club_id) REFERENCES club(id)
);