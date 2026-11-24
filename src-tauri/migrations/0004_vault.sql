-- Personal data, encrypted with Windows DPAPI before it reaches this table.
CREATE TABLE vault (
    key   TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL
) WITHOUT ROWID;
