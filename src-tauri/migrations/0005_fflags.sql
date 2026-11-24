-- FFlag profiles, stored as one validated document so every change is atomic.
CREATE TABLE fflag_state (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    document TEXT NOT NULL
);

-- Automatic snapshots of the document, newest kept.
CREATE TABLE fflag_backups (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    reason     TEXT NOT NULL,
    document   TEXT NOT NULL
);
