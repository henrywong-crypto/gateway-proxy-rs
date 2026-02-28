ALTER TABLE sessions ADD COLUMN websearch_tool_names TEXT NOT NULL DEFAULT 'WebSearch';
ALTER TABLE sessions ADD COLUMN webfetch_tool_names TEXT NOT NULL DEFAULT 'WebFetch';
