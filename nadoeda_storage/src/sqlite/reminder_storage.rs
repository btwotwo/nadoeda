mod model;

use async_trait::async_trait;
use model::{ReminderStorageModel, convert_state};
use nadoeda_models::{
    reminder::{Reminder, ReminderId, ReminderState},
    user::UserId,
};
use thiserror::Error;

use crate::reminder::{NewReminder, ReminderStorage};

#[derive(Debug, Error)]
pub enum SqliteReminderError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub struct SqliteReminderStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteReminderStorage {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReminderStorage for SqliteReminderStorage {
    type Error = SqliteReminderError;

    async fn get(&self, id: ReminderId) -> Result<Option<Reminder>, Self::Error> {
        let reminder = sqlx::query_as!(
            ReminderStorageModel,
            "SELECT * FROM reminders WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(reminder.map(Into::into))
    }
    async fn get_all_user_reminders(&self, user_id: UserId) -> Result<Vec<Reminder>, Self::Error> {
        let reminders = sqlx::query_as!(
            ReminderStorageModel,
            "SELECT * FROM reminders WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(reminders.into_iter().map(Into::into).collect())
    }
    async fn insert(&self, reminder: NewReminder) -> Result<Reminder, Self::Error> {
        let NewReminder {
            text,
            fire_at,
            user_id,
        } = reminder;
        let (state_kind, attempts_left) = convert_state(ReminderState::Pending);
        let fire_at = fire_at.into_string();

        let created_reminder = sqlx::query_as!(
            ReminderStorageModel,
            "INSERT INTO reminders (user_id, state_kind, attempts_left, fire_at, text)
VALUES (?, ?, ?, ?, ?) RETURNING *",
            user_id,
            state_kind,
            attempts_left,
            fire_at,
            text
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(created_reminder.into())
    }

    async fn update(&self, reminder: Reminder) -> Result<Reminder, Self::Error> {
        let ReminderStorageModel {
            id,
            user_id: _,
            state_kind,
            attempts_left,
            fire_at,
            text,
        } = reminder.into();
        let updated_reminder = sqlx::query_as!(
            ReminderStorageModel,
            "
UPDATE reminders
SET state_kind = ?,
    attempts_left = ?,
    fire_at = ?,
    text = ?
WHERE id = ?
RETURNING *
",
            state_kind,
            attempts_left,
            fire_at,
            text,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(updated_reminder.into())
    }
}
