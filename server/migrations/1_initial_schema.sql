-- Initial schema for LibreChat persistence layer (Phase 2).
--
-- Tables:
--   conversations    – chat threads
--   messages       – individual messages within a conversation
--   user_settings  – global user preferences (single-row table)
--   model_preferences – selected provider / model configuration (single-row table)

-- Chat threads
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY,
    title TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Messages belonging to a conversation
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- Global user settings (single row, id is pinned to 1)
CREATE TABLE IF NOT EXISTS user_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    theme TEXT DEFAULT 'system',
    language TEXT DEFAULT 'en'
);

-- Model / provider preferences (single row, id is pinned to 1)
CREATE TABLE IF NOT EXISTS model_preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    provider TEXT,
    model TEXT,
    temperature REAL,
    max_tokens INTEGER
);

-- Index for efficient message lookups by conversation
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);

-- Trigger to auto-update updated_at on conversation changes
-- Trigger to auto-update updated_at on conversation changes
CREATE TRIGGER IF NOT EXISTS update_conversations_timestamp
    AFTER UPDATE ON conversations
    FOR EACH ROW
    WHEN OLD.updated_at IS NEW.updated_at
BEGIN
    UPDATE conversations SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;
