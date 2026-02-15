CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target_url TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
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
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
