-- Add migration script here
CREATE TABLE thread_likes (
    thread_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    PRIMARY KEY (thread_id, user_id),
    FOREIGN KEY (thread_id) REFERENCES thread(id),
    FOREIGN KEY (user_id) REFERENCES user_profile(id)
);