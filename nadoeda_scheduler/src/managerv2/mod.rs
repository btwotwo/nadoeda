mod simple_reminder_scheduler;

use async_trait::async_trait;

use nadoeda_models::reminder::{Reminder, ReminderId};

pub struct ScheduleRequest {
    reminder: Reminder,
}

pub struct ScheduledReminder {
    id: ReminderId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReminderMessageType {
    Scheduled,
    Fired,
    Nag,
    Confirmation,
    Timeout,
    Finished,
    Stopped,
}

#[async_trait]
pub trait ReminderDeliveryChannel: Send + Sync + 'static {
    async fn send_reminder_notification(&self, reminder: &Reminder, message: ReminderMessageType);
}

pub trait ReminderSchedulerV2: Send + Sync + 'static {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
        delivery_channel: Box<dyn ReminderDeliveryChannel>,
    ) -> anyhow::Result<ScheduledReminder>;

    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()>;
}
