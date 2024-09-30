-- Add migration script here
CREATE TABLE announcement_media (
    announcement_id INTEGER PRIMARY KEY,
    media_id INTEGER NOT NULL,
    FOREIGN KEY (announcement_id) REFERENCES announcement(id),
    FOREIGN KEY (media_id) REFERENCES upload(id)
);