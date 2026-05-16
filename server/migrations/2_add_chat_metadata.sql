-- Add model/provider to conversations and sequence/is_error to messages.

ALTER TABLE conversations ADD COLUMN model TEXT;
ALTER TABLE conversations ADD COLUMN provider TEXT;

ALTER TABLE messages ADD COLUMN sequence INTEGER DEFAULT 0;
ALTER TABLE messages ADD COLUMN is_error INTEGER DEFAULT 0;

-- Trigger to bump conversation updated_at when a new message is inserted.
CREATE TRIGGER IF NOT EXISTS update_conversations_timestamp_on_message
    AFTER INSERT ON messages
    FOR EACH ROW
BEGIN
    UPDATE conversations SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.conversation_id;
END;

-- Index to speed up list_conversations ordering by updated_at.
CREATE INDEX IF NOT EXISTS idx_conversations_updated_at ON conversations(updated_at);
