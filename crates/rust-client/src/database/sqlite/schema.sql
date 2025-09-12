-- Miden Private Transport Client Database Schema

-- Table for storing fetched note IDs
-- This table tracks which notes have been fetched from the transport layer
-- to avoid duplicate processing and enable efficient querying by tag.
CREATE TABLE IF NOT EXISTS fetched_notes (
    note_id BLOB NOT NULL,
    tag INTEGER NOT NULL,
    fetched_at TEXT NOT NULL,

    PRIMARY KEY (note_id)
) STRICT;

-- Table for storing encrypted notes
CREATE TABLE IF NOT EXISTS stored_notes (
    note_id BLOB NOT NULL,
    tag INTEGER NOT NULL,
    header BLOB NOT NULL,
    details BLOB NOT NULL,
    created_at INTEGER NOT NULL,

    PRIMARY KEY (note_id)
) STRICT;

-- Table for storing different settings in run-time, which need to persist over runs.
CREATE TABLE IF NOT EXISTS settings (
    name  TEXT NOT NULL,
    value ANY,

    PRIMARY KEY (name),
    CONSTRAINT settings_name_is_not_empty CHECK (length(name) > 0)
) STRICT, WITHOUT ROWID;

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_fetched_notes_tag ON fetched_notes(tag);
CREATE INDEX IF NOT EXISTS idx_fetched_notes_fetched_at ON fetched_notes(fetched_at);
CREATE INDEX IF NOT EXISTS idx_stored_notes_tag ON stored_notes(tag);
CREATE INDEX IF NOT EXISTS idx_stored_notes_created_at ON stored_notes(created_at);
