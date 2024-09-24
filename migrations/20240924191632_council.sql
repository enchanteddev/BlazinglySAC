-- Add migration script here
CREATE TABLE council (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    seceretary_name VARCHAR(255) NOT NULL,
    deputy_seceretaries_name VARCHAR(255)[] NOT NULL
);