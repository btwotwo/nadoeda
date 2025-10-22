PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
       id               INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
       timezone         TEXT NOT NULL,
       tg_chat_id       INTEGER UNIQUE
);



CREATE TABLE IF NOT EXISTS reminders (
       id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
       user_id         INTEGER NOT NULL,
       state_kind      TEXT NOT NULL,
       attempts_left   INTEGER NULL,
       fire_at         TEXT NOT NULL,  -- stored as HH:MM:SS
       text            TEXT NOT NULL,
    
       FOREIGN KEY (user_id)
       REFERENCES users(id)
       ON DELETE CASCADE
       ON UPDATE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_reminders_user_id ON reminders(user_id);
CREATE INDEX IF NOT EXISTS idx_reminders_state_kind ON reminders(state_kind);
CREATE INDEX IF NOT EXISTS idx_users_id ON users(id);
