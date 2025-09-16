CREATE TABLE notes (
    id BLOB PRIMARY KEY,
    tag INTEGER NOT NULL,
    header BLOB NOT NULL,
    details BLOB NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_notes_tag ON notes(tag);
CREATE INDEX idx_notes_created_at ON notes(created_at);

