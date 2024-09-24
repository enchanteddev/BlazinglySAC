-- Add migration script here
CREATE TABLE auth (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    password VARCHAR(255) NOT NULL
);