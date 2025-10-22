use async_trait::async_trait;

use nadoeda_models::reminder::{Reminder, ReminderId};

pub struct ScheduleRequest {
    pub reminder: Reminder,
}

impl ScheduleRequest {
    pub fn new(reminder: Reminder) -> Self {
        Self { reminder }
    }
}

pub struct ScheduledReminder {
    pub id: ReminderId,
}

impl ScheduledReminder {
    #[cfg(feature = "test-util")]
    pub fn new(id: ReminderId) -> Self {
        Self { id }
    }
}

#[async_trait]
pub trait ReminderScheduler: Send + Sync + 'static {
    async fn schedule_reminder(
        &self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<ScheduledReminder>;

    async fn cancel_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()>;

    async fn acknowledge_reminder(
        &self,
        scheduled_reminder: &ScheduledReminder,
    ) -> anyhow::Result<()>;

    async fn confirm_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()>;
}
