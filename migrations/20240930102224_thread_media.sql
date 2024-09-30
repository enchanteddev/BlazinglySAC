-- Add migration script here
CREATE TABLE thread_media (
    thread_id INTEGER PRIMARY KEY,
    media_id INTEGER NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES thread(id),
    FOREIGN KEY (media_id) REFERENCES upload(id)
);