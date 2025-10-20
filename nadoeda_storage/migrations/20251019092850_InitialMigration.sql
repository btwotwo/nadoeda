PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    timezone        TEXT NOT NULL,
    tg_chat_id      INTEGER UNIQUE
);
