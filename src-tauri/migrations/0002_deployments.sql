CREATE TABLE deployments (
    version      TEXT PRIMARY KEY NOT NULL,
    install_path TEXT NOT NULL,
    source_host  TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    installed_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;
