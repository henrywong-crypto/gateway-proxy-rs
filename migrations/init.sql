CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target_url TEXT NOT NULL,
    tls_verify_disabled INTEGER NOT NULL DEFAULT 0,
    auth_header TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    headers_json TEXT,
    body_json TEXT,
    truncated_json TEXT,
    model TEXT,
    tools_json TEXT,
    messages_json TEXT,
    system_json TEXT,
    params_json TEXT,
    note TEXT,
    response_status INTEGER,
    response_headers_json TEXT,
    response_body TEXT,
    response_events_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS system_filters (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
