-- Add migration script here
CREATE TABLE event (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    club_id INTEGER NOT NULL,
    starts_at TIMESTAMP WITH TIME ZONE NOT NULL,
    venue VARCHAR(255) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user_profile(id),
    FOREIGN KEY (club_id) REFERENCES club(id)
);  