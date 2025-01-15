-- Add migration script here
CREATE TABLE website_grievance (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    grievance TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc')
);