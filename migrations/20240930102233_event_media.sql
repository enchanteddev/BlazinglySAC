-- Add migration script here
CREATE TABLE event_media (
    event_id INTEGER PRIMARY KEY,
    media_id INTEGER NOT NULL,
    FOREIGN KEY (event_id) REFERENCES event(id),
    FOREIGN KEY (media_id) REFERENCES upload(id)
);