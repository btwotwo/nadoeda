use async_trait::async_trait;

use crate::reminder::{Reminder, ReminderId};

pub struct ScheduledReminder {
    id: ReminderId
}

#[async_trait]
pub trait ReminderWorkerV2: Send + 'static {
    async fn handle_reminder(&self, reminder: &Reminder) -> anyhow::Result<()>;
}

pub trait ReminderSchedulerV2 {
    fn schedule_reminder(&mut self, reminder: Reminder, worker: impl ReminderWorkerV2) -> anyhow::Result<ScheduledReminder>;
    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()>;
}
