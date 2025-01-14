-- Add migration script here
ALTER TABLE club
    ADD COLUMN phones VARCHAR(25)[] NOT NULL DEFAULT '{}';

UPDATE club 
SET phones = ARRAY[phone];

ALTER TABLE club
    DROP COLUMN phone;

