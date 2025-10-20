PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timezone        TEXT NOT NULL,
    tg_chat_id      INTEGER NOT NULL UNIQUE
);

CREATE TRIGGER users_updated_at
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;
