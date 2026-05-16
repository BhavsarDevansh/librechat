-- Consolidate settings into a single app_settings table (Issue #29).
-- The old user_settings and model_preferences tables from migration 1
-- were never used by the application and are replaced by this schema.

DROP TABLE IF EXISTS user_settings;
DROP TABLE IF EXISTS model_preferences;

-- Single-row application settings table (id pinned to 1)
CREATE TABLE IF NOT EXISTS app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    api_endpoint TEXT NOT NULL DEFAULT '',
    auth_key TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'llama3',
    temperature REAL,
    max_tokens INTEGER,
    sidebar_collapsed INTEGER NOT NULL DEFAULT 0
);
