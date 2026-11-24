CREATE TABLE recent_experiences (
    universe_id    INTEGER PRIMARY KEY NOT NULL,
    place_id       INTEGER NOT NULL,
    name           TEXT NOT NULL,
    creator_name   TEXT,
    thumbnail_url  TEXT,
    last_played_at INTEGER NOT NULL,
    play_count     INTEGER NOT NULL DEFAULT 1,
    total_seconds  INTEGER NOT NULL DEFAULT 0
) WITHOUT ROWID;

CREATE INDEX idx_recent_experiences_last_played ON recent_experiences (last_played_at DESC);
