-- Add migration script here
ALTER TABLE user_profile
ADD COLUMN active BOOLEAN;

UPDATE user_profile
SET active = TRUE;

ALTER TABLE user_profile
ALTER COLUMN active SET DEFAULT FALSE;

ALTER TABLE user_profile
ALTER COLUMN active SET NOT NULL;