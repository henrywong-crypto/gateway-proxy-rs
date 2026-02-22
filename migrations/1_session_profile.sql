-- Add is_default flag to filter_profiles
ALTER TABLE filter_profiles ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;

-- Add profile_id to sessions
ALTER TABLE sessions ADD COLUMN profile_id TEXT REFERENCES filter_profiles(id);

-- Set is_default = 1 on the profile matching the current active_profile_id setting
UPDATE filter_profiles
SET is_default = 1
WHERE id = (SELECT value FROM settings WHERE key = 'active_profile_id');

-- Assign all existing sessions to the default profile
UPDATE sessions
SET profile_id = (SELECT id FROM filter_profiles WHERE is_default = 1 LIMIT 1);
