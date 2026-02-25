-- Add error injection column to sessions
ALTER TABLE sessions ADD COLUMN error_inject TEXT;
