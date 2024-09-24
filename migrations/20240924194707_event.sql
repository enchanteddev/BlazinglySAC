-- Add migration script here
CREATE TABLE event (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user_profile(id)
);  