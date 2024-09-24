-- Add migration script here
CREATE TABLE club (
    id SERIAL PRIMARY KEY,
    description VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) UNIQUE NOT NULL,
    club_heads TEXT[] NOT NULL,
    phone VARCHAR(25) NOT NULL,
    council_id INTEGER NOT NULL,
    FOREIGN KEY (council_id) REFERENCES council(id)
);