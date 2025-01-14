-- Add migration script here
ALTER TABLE club ADD COLUMN logo_id INTEGER REFERENCES upload(id);
