-- Add migration script here
CREATE TABLE upload (
    id SERIAL PRIMARY KEY,
    file_type VARCHAR(255) NOT NULL,
    blob BYTEA NOT NULL,
    original_hash VARCHAR(255) NOT NULL UNIQUE,
    compressed_hash VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);