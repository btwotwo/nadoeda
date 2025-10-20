PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timezone        TEXT NOT NULL,
    tg_chat_id      INTEGER
);
