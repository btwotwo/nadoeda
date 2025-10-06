mod simple_reminder_scheduler;

use async_trait::async_trait;

use nadoeda_models::reminder::{Reminder, ReminderId};

pub struct ScheduleRequest {
    reminder: Reminder,
}

pub struct ScheduledReminder {
    id: ReminderId,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReminderMessageType {
    Scheduled,
    Fired,
    Nag,
    Confirmation,
    Acknowledge,
    Timeout,
    Finished,
    Stopped,
}

#[async_trait]
pub trait ReminderDeliveryChannel: Send + Sync + 'static {
    async fn send_reminder_notification(&self, reminder: &Reminder, message: ReminderMessageType);
}

#[async_trait]
pub trait ReminderSchedulerV2: Send + Sync + 'static {
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
