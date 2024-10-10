-- Add migration script here
CREATE TABLE admin (
    id INTEGER PRIMARY KEY,
    FOREIGN KEY (id) REFERENCES user_profile(id)
);