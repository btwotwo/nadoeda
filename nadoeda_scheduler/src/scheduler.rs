use async_trait::async_trait;

use nadoeda_models::reminder::{Reminder, ReminderId};


pub struct ScheduleRequest {
    pub(crate) reminder: Reminder,
}

pub struct ScheduledReminder {
    pub(crate) id: ReminderId,
}

#[async_trait]
pub trait ReminderScheduler: Send + Sync + 'static {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<ScheduledReminder>;

    async fn cancel_reminder(
        &mut self,
        scheduled_reminder: ScheduledReminder,
    ) -> anyhow::Result<()>;

    async fn acknowledge_reminder(
        &mut self,
        scheduled_reminder: ScheduledReminder,
    ) -> anyhow::Result<()>;
}
