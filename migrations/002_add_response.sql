ALTER TABLE requests ADD COLUMN response_status INTEGER;
ALTER TABLE requests ADD COLUMN response_headers_json TEXT;
ALTER TABLE requests ADD COLUMN response_body TEXT;
ALTER TABLE requests ADD COLUMN response_events_json TEXT;
