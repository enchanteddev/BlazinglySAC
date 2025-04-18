-- Add migration script here
CREATE TABLE comment (
    id SERIAL PRIMARY KEY,
    content TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL,
    likes INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc'),
    FOREIGN KEY (user_id) REFERENCES user_profile(id),
    FOREIGN KEY (thread_id) REFERENCES thread(id)
);