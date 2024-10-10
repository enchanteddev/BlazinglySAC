-- Add migration script here
CREATE TABLE user_profile (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL
);

-- -- Add migration script here
-- CREATE TABLE auth (
--     email VARCHAR(255) NOT NULL PRIMARY KEY,
--     password VARCHAR(255) NOT NULL
-- );