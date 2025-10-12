pub mod simple_reminder_scheduler;

use async_trait::async_trait;

use nadoeda_models::reminder::{Reminder, ReminderId};

use crate::ReminderDeliveryChannel;

pub struct ScheduleRequest {
    reminder: Reminder,
}

pub struct ScheduledReminder {
    id: ReminderId,
}


#[async_trait]
pub trait ReminderScheduler: Send + Sync + 'static {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
        delivery_channel: impl ReminderDeliveryChannel,
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
