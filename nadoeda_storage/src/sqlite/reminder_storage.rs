mod model;

use async_trait::async_trait;
use nadoeda_models::{reminder::{Reminder, ReminderId}, user::UserId};
use model::ReminderStorageModel;
use crate::{reminder::NewReminder, ReminderStorage};


pub struct SqliteReminderStorage {
    pool: sqlx::SqlitePool,
}

#[async_trait]
impl ReminderStorage for SqliteReminderStorage {
    type Error = anyhow::Error;
    
    async fn get(&self, id: ReminderId) -> Result<Option<Reminder>, Self::Error> {
        let reminder = sqlx::query_as!(ReminderStorageModel,
            "SELECT * FROM reminders WHERE id = ?", id).fetch_optional(&self.pool).await?;
        todo!()
    }
    async fn get_all_user_reminders(&self, user_id: UserId) -> Result<Vec<Reminder>, Self::Error> {
        todo!()
    }
    async fn insert(&self, reminder: NewReminder) -> Result<Reminder, Self::Error> {
        todo!()
    }
    async fn update(&self, reminder: Reminder) -> Result<Reminder, Self::Error> {
        todo!()
    }
}
