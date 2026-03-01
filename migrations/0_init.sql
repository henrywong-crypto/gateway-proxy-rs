CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target_url TEXT NOT NULL,
    tls_verify_disabled INTEGER NOT NULL DEFAULT 0,
    auth_header TEXT,
    x_api_key TEXT,
    profile_id TEXT REFERENCES filter_profiles(id),
    error_inject TEXT,
    webfetch_intercept INTEGER NOT NULL DEFAULT 0,
    webfetch_whitelist TEXT,
    webfetch_tool_names TEXT NOT NULL DEFAULT 'WebFetch',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
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
    webfetch_first_response_body TEXT,
    webfetch_first_response_events_json TEXT,
    webfetch_followup_body_json TEXT,
    webfetch_rounds_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS filter_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS system_filters (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES filter_profiles(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tool_filters (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES filter_profiles(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS message_filters (
    profile_id TEXT PRIMARY KEY REFERENCES filter_profiles(id) ON DELETE CASCADE,
    keep_tool_pairs INTEGER NOT NULL DEFAULT 0
);
