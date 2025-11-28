use std::sync::Arc;

use async_trait::async_trait;
use nadoeda_scheduler::{ReminderScheduler, ScheduleRequest, ScheduledReminder};
use nadoeda_storage::sqlite::{reminder_storage::SqliteReminderStorage, user_storage::SqliteUserInfoStorage};
use sqlx::{Pool, Sqlite};

pub struct NoopReminderScheduler;
#[async_trait]
impl ReminderScheduler for NoopReminderScheduler {
    async fn schedule_reminder(
        &self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<ScheduledReminder> {
        Ok(ScheduledReminder::new(1))
    }

    async fn cancel_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        Ok(())
    }

    async fn acknowledge_reminder(
        &self,
        scheduled_reminder: &ScheduledReminder,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn confirm_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn storage(pool: Pool<Sqlite>) -> Arc<SqliteReminderStorage> {
    Arc::new(SqliteReminderStorage::new(pool))
}

pub fn user_storage(pool: Pool<Sqlite>) -> Arc<SqliteUserInfoStorage> {
    Arc::new(SqliteUserInfoStorage::new(pool))
}
