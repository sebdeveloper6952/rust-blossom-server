CREATE TABLE IF NOT EXISTS blobs
(
    pubkey TEXT NOT NULL,
    hash TEXT NOT NULL PRIMARY KEY,
    blob BLOB NOT NULL,
    type TEXT NOT NULL,
    size INT NOT NULL,
    created INT NOT NULL
);
