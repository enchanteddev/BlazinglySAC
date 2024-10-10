-- Add migration script here
CREATE TABLE council (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    secretary_name VARCHAR(255) NOT NULL,
    deputy_secretaries_name VARCHAR(255)[] NOT NULL
);