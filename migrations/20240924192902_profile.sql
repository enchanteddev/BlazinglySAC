-- Add migration script here
CREATE TABLE user_profile (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    FOREIGN KEY (email) REFERENCES auth(email)
);